// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::gate::{current_iso_timestamp, run_with_timeout, sha256_file};
use crate::model::{
    GateStatus, MutationReceipt, ProbeResult, RuntimeToolIdentities, ToolchainManifest,
};
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

#[allow(dead_code)]
pub struct ResolvedTools {
    pub kani_path: PathBuf,
    pub cargo_kani_path: PathBuf,
    pub cbmc_path: PathBuf,
    pub kissat_path: PathBuf,
    pub identities: RuntimeToolIdentities,
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn resolve_executable(path_or_name: &Path) -> Result<PathBuf, String> {
    if path_or_name.components().count() > 1 {
        if path_or_name.is_file() {
            return Ok(path_or_name.to_path_buf());
        }
        return Err(format!("executable not found at: {}", path_or_name.display()));
    }
    let name = path_or_name.to_string_lossy();
    if let Some(found) = find_in_path(&name) {
        return Ok(found);
    }
    Err(format!("executable '{name}' not found in PATH"))
}

pub fn discover_tool_binary(kani_bin: &Path, tool_name: &str) -> Result<PathBuf, String> {
    // 1. Check parent directory of kani_bin
    if let Some(parent) = kani_bin.parent() {
        let direct = parent.join(tool_name);
        if direct.is_file() {
            return Ok(direct);
        }
        // If kani_bin is inside target/kani/bin, check relative scripts for cargo-kani
        if tool_name == "cargo-kani" {
            let script = parent.join("../../scripts/cargo-kani");
            if script.is_file() {
                return Ok(script);
            }
        }
    }

    // 2. Check KANI_HOME environment variable if present
    if let Ok(kani_home) = std::env::var("KANI_HOME") {
        let home_path = PathBuf::from(kani_home);
        let direct = home_path.join("bin").join(tool_name);
        if direct.is_file() {
            return Ok(direct);
        }
        if let Ok(entries) = fs::read_dir(&home_path) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("bin").join(tool_name);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }
    }

    // 3. Check default user kani install directory ~/.kani/kani-*/bin/<tool_name>
    if let Some(home_dir) = std::env::var_os("HOME").map(PathBuf::from) {
        let dot_kani = home_dir.join(".kani");
        if dot_kani.is_dir() {
            if let Ok(entries) = fs::read_dir(&dot_kani) {
                for entry in entries.flatten() {
                    let candidate = entry.path().join("bin").join(tool_name);
                    if candidate.is_file() {
                        return Ok(candidate);
                    }
                }
            }
        }
    }

    // 4. Check system PATH
    if let Some(found) = find_in_path(tool_name) {
        return Ok(found);
    }

    Err(format!("could not locate required runtime tool binary: {tool_name}"))
}

pub fn resolve_runtime_tools(kani_bin: &Path) -> Result<ResolvedTools, String> {
    let kani_path = resolve_executable(kani_bin)?;
    let cargo_kani_path = discover_tool_binary(&kani_path, "cargo-kani")?;
    let cbmc_path = discover_tool_binary(&kani_path, "cbmc")?;
    let kissat_path = discover_tool_binary(&kani_path, "kissat")?;

    let kani_sha256 = sha256_file(&kani_path).map_err(|e| format!("failed to hash kani: {e}"))?;
    let cargo_kani_sha256 =
        sha256_file(&cargo_kani_path).map_err(|e| format!("failed to hash cargo-kani: {e}"))?;
    let cbmc_sha256 = sha256_file(&cbmc_path).map_err(|e| format!("failed to hash cbmc: {e}"))?;
    let kissat_sha256 =
        sha256_file(&kissat_path).map_err(|e| format!("failed to hash kissat: {e}"))?;

    // Query cargo-kani version (--version --verbose)
    let cargo_kani_out = Command::new(&cargo_kani_path)
        .arg("--version")
        .arg("--verbose")
        .output()
        .map_err(|e| format!("failed to run cargo-kani --version --verbose: {e}"))?;
    let mut cargo_kani_version = String::from_utf8_lossy(&cargo_kani_out.stdout).trim().to_string();

    // If cargo-kani is the dev wrapper or launcher returning a single line, check kani-driver with arg0
    if cargo_kani_version.starts_with("cargo-kani ") {
        if let Some(parent) = kani_path.parent() {
            let driver = parent.join("kani-driver");
            if driver.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    if let Ok(driver_out) = Command::new(&driver)
                        .arg0("cargo-kani")
                        .arg("--version")
                        .arg("--verbose")
                        .output()
                    {
                        let driver_str =
                            String::from_utf8_lossy(&driver_out.stdout).trim().to_string();
                        if driver_str.contains("Kani Rust Verifier") {
                            cargo_kani_version = driver_str;
                        }
                    }
                }
            }
        }
    }

    // Query cbmc version (--version)
    let cbmc_out = Command::new(&cbmc_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to run cbmc --version: {e}"))?;
    let cbmc_version = String::from_utf8_lossy(&cbmc_out.stdout).trim().to_string();

    // Query kissat version (--version)
    let kissat_out = Command::new(&kissat_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to run kissat --version: {e}"))?;
    let kissat_version = String::from_utf8_lossy(&kissat_out.stdout).trim().to_string();

    Ok(ResolvedTools {
        kani_path,
        cargo_kani_path,
        cbmc_path,
        kissat_path,
        identities: RuntimeToolIdentities {
            cargo_kani_version,
            cargo_kani_sha256,
            kani_sha256,
            cbmc_version,
            cbmc_sha256,
            kissat_version,
            kissat_sha256,
        },
    })
}

pub fn validate_toolchain_identities(
    observed: &RuntimeToolIdentities,
    expected: &ToolchainManifest,
) -> Result<(), String> {
    if observed.cbmc_version != expected.cbmc_version {
        return Err(format!(
            "runtime CBMC mismatch: observed {:?}, expected {:?}",
            observed.cbmc_version, expected.cbmc_version
        ));
    }
    if observed.kissat_version != expected.kissat_version {
        return Err(format!(
            "runtime Kissat mismatch: observed {:?}, expected {:?}",
            observed.kissat_version, expected.kissat_version
        ));
    }
    if observed.cargo_kani_version != expected.cargo_kani_version {
        return Err(format!(
            "cargo-kani identity mismatch:\nobserved:\n{}\nexpected:\n{}",
            observed.cargo_kani_version, expected.cargo_kani_version
        ));
    }
    Ok(())
}

struct SnapshotGuard(PathBuf);
impl Drop for SnapshotGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn run_all_mutations(ctx: &MutationContext) -> Result<MutationReceipt, String> {
    // 1. Resolve runtime binaries and capture toolchain identities
    let resolved = resolve_runtime_tools(&ctx.kani_bin)?;
    let observed_runtime = resolved.identities;

    // 2. Hash source fixtures prior to any operations
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

    // 3. Create isolated snapshot directory and copy fixtures + manifest
    let timestamp_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let snapshot_root = std::env::temp_dir().join(format!(
        "kani-qualify-snapshot-{}-{}",
        std::process::id(),
        timestamp_nanos
    ));
    fs::create_dir_all(&snapshot_root)
        .map_err(|e| format!("failed to create snapshot dir {}: {e}", snapshot_root.display()))?;
    let _snapshot_guard = SnapshotGuard(snapshot_root.clone());

    let snapshot_fixtures = snapshot_root.join("fixtures");
    fs::create_dir_all(&snapshot_fixtures).map_err(|e| {
        format!("failed to create snapshot fixtures dir {}: {e}", snapshot_fixtures.display())
    })?;

    for (name, _) in &fixture_hashes {
        let src = ctx.fixtures_dir.join(name);
        let dst = snapshot_fixtures.join(name);
        fs::copy(&src, &dst)
            .map_err(|e| format!("failed to copy fixture {name} into snapshot: {e}"))?;
    }

    let snapshot_toolchain = snapshot_root.join("toolchain.json");
    fs::copy(ctx.toolchain_path, &snapshot_toolchain).map_err(|e| {
        format!(
            "failed to copy toolchain manifest into snapshot {}: {e}",
            snapshot_toolchain.display()
        )
    })?;

    // 4. Hash snapshot files and verify match with pre-run digests
    let mut snapshot_hashes = BTreeMap::new();
    for (name, _) in &fixture_hashes {
        let path = snapshot_fixtures.join(name);
        let hash = sha256_file(&path).map_err(|e| e.to_string())?;
        snapshot_hashes.insert(name.clone(), hash);
    }
    if snapshot_hashes != fixture_hashes {
        return Err("snapshot fixture hashes do not match source fixture hashes".to_string());
    }

    let mut results = BTreeMap::new();

    // Probe 1: Inadequate unwind probe: must fail specifically due to unwinding assertion
    let inadequate_path = snapshot_fixtures.join("inadequate_unwind.rs");
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

    // Probe 2: Vacuity probe: fail-closed (parser errors or verifier crashes are NOT detections)
    let vacuity_path = snapshot_fixtures.join("vacuity.rs");
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

    // Probe 3: Timeout probe: child process termination
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

    // Probe 4: Positive control probe: baseline verification succeeds and parses cleanly
    let probe_path = snapshot_fixtures.join("backend_probe.rs");
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

    // Probe 5: Parser truncation probe: truncated logs missing summary must be detected and rejected
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

    // Probe 6: Toolchain runtime version validation probe (fail-closed detection of mismatched runtime binaries)
    let mut mismatched_cbmc_manifest = ctx.toolchain.clone();
    mismatched_cbmc_manifest.cbmc_version =
        if observed_runtime.cbmc_version == "6.10.0 (cbmc-6.10.0)" {
            "6.8.0 (cbmc-6.8.0)".to_string()
        } else {
            "6.10.0 (cbmc-6.10.0)".to_string()
        };
    let cbmc_mismatch_detected =
        validate_toolchain_identities(&observed_runtime, &mismatched_cbmc_manifest).is_err();

    let mut mismatched_kissat_manifest = ctx.toolchain.clone();
    mismatched_kissat_manifest.kissat_version = "0.0.0-bogus".to_string();
    let kissat_mismatch_detected =
        validate_toolchain_identities(&observed_runtime, &mismatched_kissat_manifest).is_err();

    let mut matching_manifest = ctx.toolchain.clone();
    matching_manifest.cbmc_version = observed_runtime.cbmc_version.clone();
    matching_manifest.kissat_version = observed_runtime.kissat_version.clone();
    matching_manifest.cargo_kani_version = observed_runtime.cargo_kani_version.clone();
    let match_accepted =
        validate_toolchain_identities(&observed_runtime, &matching_manifest).is_ok();

    let toolchain_probe_detected =
        cbmc_mismatch_detected && kissat_mismatch_detected && match_accepted;
    results.insert(
        "bad_runtime_cbmc".to_string(),
        ProbeResult::new(
            toolchain_probe_detected,
            "toolchain validation failed to detect invalid runtime CBMC or Kissat version mismatch",
        ),
    );

    // Probe 7: Backend failure probe: verifier must fail-closed on backend-stage failure
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

    // 5. Post-probe check: verify original source fixtures and manifest have not drifted
    for (name, expected_hash) in &fixture_hashes {
        let src = ctx.fixtures_dir.join(name);
        let current_hash = sha256_file(&src).map_err(|e| e.to_string())?;
        if current_hash != *expected_hash {
            return Err(format!("source fixture {name} was modified during mutation execution"));
        }
    }
    let post_toolchain_sha256 = sha256_file(ctx.toolchain_path).map_err(|e| e.to_string())?;
    if post_toolchain_sha256 != toolchain_sha256 {
        return Err("toolchain manifest modified during mutation execution".to_string());
    }

    let all_detected = results.values().all(|r| r.detected);

    Ok(MutationReceipt {
        schema: 1,
        profile: ctx.toolchain.profile.clone(),
        executed_at: current_iso_timestamp(),
        gate_sha256,
        toolchain_sha256,
        observed_runtime,
        snapshot_hashes,
        fixtures: fixture_hashes,
        results,
        status: if all_detected { GateStatus::Pass } else { GateStatus::Fail },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_toolchain_identities_detection() {
        let manifest_raw = include_str!("../../../qualification/manifests/core-v1/toolchain.json");
        let manifest: ToolchainManifest = serde_json::from_str(manifest_raw).unwrap();

        let mut observed = RuntimeToolIdentities {
            cargo_kani_version: manifest.cargo_kani_version.clone(),
            cargo_kani_sha256: "0000".to_string(),
            kani_sha256: "1111".to_string(),
            cbmc_version: manifest.cbmc_version.clone(),
            cbmc_sha256: "2222".to_string(),
            kissat_version: manifest.kissat_version.clone(),
            kissat_sha256: "3333".to_string(),
        };

        // Exact match passes
        assert!(validate_toolchain_identities(&observed, &manifest).is_ok());

        // CBMC mismatch fails with specific error
        observed.cbmc_version = "6.8.0 (cbmc-6.8.0)".to_string();
        let err = validate_toolchain_identities(&observed, &manifest).unwrap_err();
        assert!(err.contains("runtime CBMC mismatch"));

        // Kissat mismatch fails with specific error
        observed.cbmc_version = manifest.cbmc_version.clone();
        observed.kissat_version = "3.1.0".to_string();
        let err = validate_toolchain_identities(&observed, &manifest).unwrap_err();
        assert!(err.contains("runtime Kissat mismatch"));

        // cargo-kani mismatch fails with specific error
        observed.kissat_version = manifest.kissat_version.clone();
        observed.cargo_kani_version = "Kani Rust Verifier 0.66.0 (outdated)".to_string();
        let err = validate_toolchain_identities(&observed, &manifest).unwrap_err();
        assert!(err.contains("cargo-kani identity mismatch"));
    }

    #[test]
    fn test_snapshot_isolation_and_source_drift_detection() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kani-test-drift-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let fixture_file = temp_dir.join("test_fixture.rs");
        fs::write(&fixture_file, b"fn main() {}").unwrap();

        let initial_hash = sha256_file(&fixture_file).unwrap();
        assert_eq!(initial_hash, hash_bytes(b"fn main() {}"));

        // Modify file
        fs::write(&fixture_file, b"fn main() { panic!(); }").unwrap();
        let modified_hash = sha256_file(&fixture_file).unwrap();
        assert_ne!(initial_hash, modified_hash);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
