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
    pub toolchain_sha256: String,
    pub toolchain: ToolchainManifest,
}

pub fn run_all_mutations(ctx: &MutationContext) -> Result<MutationReceipt, String> {
    // Hash inputs before executing any probes
    let mut fixture_hashes = BTreeMap::new();
    for entry in fs::read_dir(ctx.fixtures_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let hash = sha256_file(&path).map_err(|e| e.to_string())?;
            fixture_hashes.insert(path.file_name().unwrap().to_string_lossy().to_string(), hash);
        }
    }
    let toolchain_sha256 = ctx.toolchain_sha256.clone();
    let gate_sha256 = match std::env::current_exe() {
        Ok(exe) => sha256_file(&exe).unwrap_or_else(|_| hash_bytes(b"kani-qualify-v0.1.0")),
        Err(_) => hash_bytes(b"kani-qualify-v0.1.0"),
    };

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

    // 4. Positive control probe: baseline verification succeeds and parses cleanly
    let probe_path = ctx.fixtures_dir.join("backend_probe.rs");
    let mut cmd = Command::new(&ctx.kani_bin);
    cmd.arg(&probe_path).arg("--output-format=terse");
    let positive = run_with_timeout(cmd, Duration::from_secs(120))?;
    let positive_code = positive.status.code();
    let complete_stdout = positive.stdout;
    let parsed_positive = parse_kani_output(&complete_stdout);
    let positive_detected =
        positive.status.success() && parsed_positive.as_ref().is_ok_and(|p| p.is_pass());
    results.insert(
        "positive_control".to_string(),
        ProbeResult {
            returncode: positive_code,
            output_sha256: Some(hash_bytes(complete_stdout.as_bytes())),
            ..ProbeResult::new(
                positive_detected,
                "positive control failed to verify or parse as valid pass",
            )
        },
    );

    // 5. Parser truncation probe: truncated logs missing summary must be detected and rejected
    let (truncation_detected, truncated_stdout) = if positive_detected {
        let truncated = complete_stdout.split("Complete -").next().unwrap_or("");
        let parsed_trunc = parse_kani_output(truncated);
        let rejected = parsed_trunc.as_ref().map_or(true, |p| !p.is_pass());
        let distinct = truncated != complete_stdout && !truncated.is_empty();
        (distinct && rejected, truncated)
    } else {
        (false, "")
    };
    results.insert(
        "parser_truncation".to_string(),
        ProbeResult {
            positive_control_returncode: positive_code,
            complete_output_sha256: Some(hash_bytes(complete_stdout.as_bytes())),
            truncated_output_sha256: Some(hash_bytes(truncated_stdout.as_bytes())),
            ..ProbeResult::new(
                truncation_detected,
                "truncated output was accepted or could not be derived from positive control",
            )
        },
    );

    // 6. Toolchain runtime version validation probe
    let manifest_valid = ctx.toolchain.validate().is_ok();
    let bogus_manifest = ToolchainManifest {
        schema: Some(999),
        profile: Some("invalid-profile".to_string()),
        kani: Some("bogus-version".to_string()),
        rustc: None,
        cbmc: Some("0.0.0-bogus".to_string()),
        kissat: None,
        extra: BTreeMap::new(),
    };
    let bogus_rejected = bogus_manifest.validate().is_err();
    let toolchain_probe_detected = manifest_valid && bogus_rejected;
    results.insert(
        "bad_runtime_cbmc".to_string(),
        ProbeResult::new(
            toolchain_probe_detected,
            "toolchain validation failed to detect invalid manifest or runtime",
        ),
    );

    // 7. Backend failure probe: verifier must fail-closed on backend-stage failure
    let mut invalid_cmd = Command::new(&ctx.kani_bin);
    invalid_cmd
        .arg(&probe_path)
        .arg("--output-format=terse")
        .arg("-Z")
        .arg("unstable-options")
        .arg("--cbmc-args")
        .arg("--unsupported-cbmc-option-probe-fail");
    let invalid_run = run_with_timeout(invalid_cmd, Duration::from_secs(60))?;
    let backend_failure_detected = !invalid_run.status.success()
        && invalid_run.status.code() != Some(2)
        && (invalid_run.stdout.contains("CBMC failed")
            || invalid_run.stdout.contains("VERIFICATION:- FAILED")
            || invalid_run.stdout.contains("Unknown option")
            || invalid_run.stderr.contains("Unknown option"));
    results.insert(
        "backend_failure".to_string(),
        ProbeResult {
            returncode: invalid_run.status.code(),
            output_sha256: Some(hash_bytes(invalid_run.stdout.as_bytes())),
            ..ProbeResult::new(
                backend_failure_detected,
                "invalid backend configuration did not produce expected backend failure",
            )
        },
    );

    // Verify no input drift occurred during probe execution
    let post_toolchain_sha256 = sha256_file(ctx.toolchain_path).map_err(|e| e.to_string())?;
    if post_toolchain_sha256 != toolchain_sha256 {
        return Err("toolchain manifest modified during mutation execution".to_string());
    }

    let all_detected = results.values().all(|r| r.detected);

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
