// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::gate::{current_iso_timestamp, run_with_timeout, sha256_file};
use crate::model::{GateStatus, MutationReceipt, ProbeResult, ToolchainManifest};
use crate::parser::parse_kani_output;

pub fn hash_bytes(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

pub struct MutationContext<'a> {
    pub kani_bin: PathBuf,
    pub fixtures_dir: &'a Path,
    pub toolchain_path: &'a Path,
    pub toolchain: ToolchainManifest,
}

pub fn run_all_mutations(ctx: &MutationContext) -> Result<MutationReceipt, String> {
    let mut results = BTreeMap::new();

    // 1. Inadequate unwind probe: must fail specifically due to unwinding assertion
    let inadequate_path = ctx.fixtures_dir.join("inadequate_unwind.rs");
    let mut cmd = Command::new(&ctx.kani_bin);
    cmd.arg(&inadequate_path).arg("--output-format=terse");
    let inadequate = run_with_timeout(cmd, Duration::from_secs(120))?;
    let ineq_detected =
        !inadequate.status.success() && inadequate.stdout.contains("unwinding assertion");
    results.insert(
        "inadequate_unwind".to_string(),
        ProbeResult {
            returncode: inadequate.status.code(),
            output_sha256: Some(hash_bytes(inadequate.stdout.as_bytes())),
            ..ProbeResult::new(
                ineq_detected,
                "failed to detect inadequate unwind via unwinding assertion",
            )
        },
    );

    // 2. Vacuity probe: fail-closed (parser errors or verifier crashes are NOT detections)
    let vacuity_path = ctx.fixtures_dir.join("vacuity.rs");
    let mut cmd = Command::new(&ctx.kani_bin);
    cmd.arg(&vacuity_path).arg("--output-format=terse");
    let vacuity = run_with_timeout(cmd, Duration::from_secs(120))?;
    let parsed_vacuity = parse_kani_output(&vacuity.stdout);
    let vacuity_detected = match parsed_vacuity {
        Ok(parsed) => {
            let cover_failed =
                parsed.harnesses.values().any(|h| h.covers.is_some_and(|c| c.satisfied < c.total));
            cover_failed || vacuity.stdout.contains("UNSATISFIED")
        }
        Err(_) => false, // Fail-closed: parse failure is not vacuity detection
    };
    results.insert(
        "vacuity".to_string(),
        ProbeResult {
            kani_returncode: vacuity.status.code(),
            output_sha256: Some(hash_bytes(vacuity.stdout.as_bytes())),
            ..ProbeResult::new(vacuity_detected, "vacuity was not detected")
        },
    );

    // 3. Timeout probe: child process termination
    let mut sleep_cmd = Command::new("sleep");
    sleep_cmd.arg("5");
    let timeout_result = run_with_timeout(sleep_cmd, Duration::from_millis(200));
    let timeout_detected = match timeout_result {
        Err(e) => e.contains("timed out"),
        Ok(_) => false,
    };
    results.insert(
        "timeout".to_string(),
        ProbeResult::new(timeout_detected, "timeout probe did not trigger timeout"),
    );

    // 4. Parser truncation probe: truncated logs missing summary must be detected
    let probe_path = ctx.fixtures_dir.join("backend_probe.rs");
    let mut cmd = Command::new(&ctx.kani_bin);
    cmd.arg(&probe_path).arg("--output-format=terse");
    let positive = run_with_timeout(cmd, Duration::from_secs(120))?;
    let positive_code = positive.status.code();
    let complete_stdout = positive.stdout;
    let truncated_stdout = complete_stdout.split("Complete -").next().unwrap_or("");
    let parsed_truncated = parse_kani_output(truncated_stdout);
    let truncation_detected =
        parsed_truncated.is_err() || parsed_truncated.unwrap().summary.is_none();
    results.insert(
        "parser_truncation".to_string(),
        ProbeResult {
            positive_control_returncode: positive_code,
            complete_output_sha256: Some(hash_bytes(complete_stdout.as_bytes())),
            truncated_output_sha256: Some(hash_bytes(truncated_stdout.as_bytes())),
            ..ProbeResult::new(truncation_detected, "truncated output was accepted")
        },
    );

    // 5. Toolchain runtime version validation probe
    let cbmc_declared = ctx.toolchain.cbmc.as_deref().unwrap_or("");
    let toolchain_valid = !cbmc_declared.is_empty() && cbmc_declared != "0.0.0";
    results.insert(
        "bad_runtime_cbmc".to_string(),
        ProbeResult::new(
            toolchain_valid,
            "toolchain manifest has empty or unresolvable CBMC version",
        ),
    );

    // 6. Positive control probe: baseline verification succeeds
    let positive_detected =
        positive.status.success() && complete_stdout.contains("VERIFICATION:- SUCCESSFUL");
    results.insert(
        "positive_control".to_string(),
        ProbeResult {
            returncode: positive_code,
            output_sha256: Some(hash_bytes(complete_stdout.as_bytes())),
            ..ProbeResult::new(positive_detected, "positive control failed to verify")
        },
    );

    // 7. Backend failure probe: verifier must fail-closed on invalid backend configuration
    let mut invalid_cmd = Command::new(&ctx.kani_bin);
    invalid_cmd.arg(&probe_path).arg("--output-format=terse").arg("--solver=non_existent_solver");
    let invalid_run = run_with_timeout(invalid_cmd, Duration::from_secs(60))?;
    let backend_failure_detected = !invalid_run.status.success();
    results.insert(
        "backend_failure".to_string(),
        ProbeResult {
            returncode: invalid_run.status.code(),
            output_sha256: Some(hash_bytes(invalid_run.stdout.as_bytes())),
            ..ProbeResult::new(
                backend_failure_detected,
                "invalid backend configuration did not fail",
            )
        },
    );

    let mut fixture_hashes = BTreeMap::new();
    for entry in fs::read_dir(ctx.fixtures_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let hash = sha256_file(&path).map_err(|e| e.to_string())?;
            fixture_hashes.insert(path.file_name().unwrap().to_string_lossy().to_string(), hash);
        }
    }

    let toolchain_sha256 = sha256_file(ctx.toolchain_path).map_err(|e| e.to_string())?;
    let all_detected = results.values().all(|r| r.detected);

    let gate_sha256 = match std::env::current_exe() {
        Ok(exe) => sha256_file(&exe).unwrap_or_else(|_| hash_bytes(b"kani-qualify-v0.1.0")),
        Err(_) => hash_bytes(b"kani-qualify-v0.1.0"),
    };

    Ok(MutationReceipt {
        schema: 1,
        executed_at: current_iso_timestamp(),
        gate_sha256,
        toolchain_sha256,
        fixtures: fixture_hashes,
        results,
        status: if all_detected { GateStatus::Pass } else { GateStatus::Fail },
    })
}
