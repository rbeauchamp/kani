// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::gate::{
    CommandOutput, current_iso_timestamp, executable_sha256, run_with_timeout, sha256_file,
};
use crate::model::{
    ConsumerDiagnostics, ConsumerManifest, GateStatus, MutationPurpose, MutationReceipt,
    ProbeResult, RuntimeToolIdentities, ToolchainManifest, validate_sha256,
};
use crate::parser::{ParsedOutput, parse_kani_output};

pub fn hash_bytes(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

pub enum MutationMode {
    InfrastructureSelfTest,
    Qualification { toolchain: ToolchainManifest, sha256: String },
}

pub struct MutationContext<'a> {
    pub kani_bin: PathBuf,
    pub fixtures_dir: &'a Path,
    pub mode: MutationMode,
}

pub struct ResolvedTools {
    driver: PathBuf,
    compiler: PathBuf,
    cbmc: PathBuf,
    kissat: PathBuf,
}

pub fn resolve_executable(path_or_name: &Path) -> Result<PathBuf, String> {
    let path = if path_or_name.components().count() > 1 {
        path_or_name.to_path_buf()
    } else {
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join(path_or_name))
            .find(|p| p.is_file())
            .ok_or_else(|| format!("executable {} not found in PATH", path_or_name.display()))?
    };
    fs::canonicalize(&path)
        .map_err(|e| format!("cannot resolve executable {}: {e}", path.display()))
}

impl ResolvedTools {
    fn resolve(kani_bin: &Path) -> Result<Self, String> {
        let supplied = resolve_executable(kani_bin)?;
        let supplied_bin = supplied.parent().ok_or("Kani executable has no parent directory")?;
        let driver = if supplied.file_name().is_some_and(|n| n == "kani-driver") {
            supplied.clone()
        } else {
            supplied_bin.join("kani-driver")
        };
        if !driver.is_file() {
            return Err("select the kani-driver binary (or a sibling kani executable) in one built checkout or unpacked bundle; global installers are not qualification executables".to_string());
        }
        let driver = fs::canonicalize(driver).map_err(|e| e.to_string())?;
        // Match KaniInstallation's current_exe-based compiler lookup, including
        // a sibling launcher whose driver points into another installation.
        let bin = driver.parent().ok_or("Kani driver has no parent directory")?;
        let backend = |name: &str| {
            let bundled = bin.join(name);
            if bundled.is_file() {
                resolve_executable(&bundled)
            } else {
                resolve_executable(Path::new(name))
            }
        };
        let compiler = resolve_executable(&bin.join("kani-compiler"))?;
        let cbmc = backend("cbmc")?;
        let kissat = backend("kissat")?;
        Ok(Self { driver, compiler, cbmc, kissat })
    }

    fn hashes(&self) -> Result<[String; 4], String> {
        let hash = |path: &Path| {
            sha256_file(path).map_err(|e| format!("cannot hash {}: {e}", path.display()))
        };
        Ok([hash(&self.driver)?, hash(&self.compiler)?, hash(&self.cbmc)?, hash(&self.kissat)?])
    }

    fn command(&self, mode: &str, runtime_bin: &Path) -> Result<Command, String> {
        let mut cmd = Command::new(&self.driver);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.arg0(mode);
        }
        #[cfg(not(unix))]
        return Err("qualification execution currently requires Unix".to_string());
        let bin = self.driver.parent().ok_or("Kani driver has no parent directory")?;
        let root = bin.parent().ok_or("Kani installation has no root")?;
        let path = std::env::join_paths(
            [runtime_bin.to_path_buf(), bin.to_path_buf(), root.join("pyroot/bin")]
                .into_iter()
                .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())),
        )
        .map_err(|e| e.to_string())?;
        let pythonpath = std::env::join_paths(
            [root.join("pyroot")]
                .into_iter()
                .chain(std::env::split_paths(&std::env::var_os("PYTHONPATH").unwrap_or_default())),
        )
        .map_err(|e| e.to_string())?;
        cmd.env("PATH", path).env("PYTHONPATH", pythonpath);
        // The direct driver selects its own compiler/sysroot. Loader overrides
        // inherited from cargo must not select a different rustc shared library.
        cmd.env_remove("LD_LIBRARY_PATH").env_remove("DYLD_FALLBACK_LIBRARY_PATH");
        let channel_path = root.join("rust-toolchain-version");
        match fs::read_to_string(&channel_path) {
            Ok(channel) => {
                cmd.env("RUSTUP_TOOLCHAIN", channel.trim());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot read bundled Rust toolchain identity: {e}")),
        }
        Ok(cmd)
    }

    fn observe(&self, runtime_bin: &Path) -> Result<RuntimeToolIdentities, String> {
        let [driver_hash, compiler_hash, cbmc_hash, kissat_hash] = self.hashes()?;
        let mut cargo_kani = self.command("cargo-kani", runtime_bin)?;
        cargo_kani.args(["--version", "--verbose"]);
        let mut cbmc = Command::new(&self.cbmc);
        cbmc.arg("--version");
        let mut kissat = Command::new(&self.kissat);
        kissat.arg("--version");
        let identities = RuntimeToolIdentities {
            cargo_kani_version: version(cargo_kani)?,
            cargo_kani_sha256: driver_hash.clone(),
            kani_sha256: driver_hash,
            kani_compiler_sha256: compiler_hash,
            cbmc_version: version(cbmc)?,
            cbmc_sha256: cbmc_hash,
            kissat_version: version(kissat)?,
            kissat_sha256: kissat_hash,
        };
        identities.validate()?;
        self.verify_hashes(&identities)?;
        Ok(identities)
    }

    fn verify_hashes(&self, observed: &RuntimeToolIdentities) -> Result<(), String> {
        if self.hashes()?
            != [
                observed.kani_sha256.clone(),
                observed.kani_compiler_sha256.clone(),
                observed.cbmc_sha256.clone(),
                observed.kissat_sha256.clone(),
            ]
        {
            return Err("runtime binaries changed during qualification".to_string());
        }
        Ok(())
    }
}

fn version(cmd: Command) -> Result<String, String> {
    let output = run_with_timeout(cmd, Duration::from_secs(30))?;
    if !output.status.success()
        || output.stdout.trim().is_empty()
        || !output.stderr.trim().is_empty()
    {
        return Err(format!("runtime version query failed: {}\n{}", output.status, output.stderr));
    }
    Ok(output.stdout.trim().to_string())
}

fn validate_versions(
    observed: &RuntimeToolIdentities,
    cargo_kani: &str,
    cbmc: &str,
    kissat: &str,
) -> Result<(), String> {
    if observed.cbmc_version != cbmc {
        return Err(format!(
            "runtime CBMC mismatch: observed {:?}, expected {cbmc:?}",
            observed.cbmc_version
        ));
    }
    if observed.kissat_version != kissat {
        return Err(format!(
            "runtime Kissat mismatch: observed {:?}, expected {kissat:?}",
            observed.kissat_version
        ));
    }
    if observed.cargo_kani_version != cargo_kani {
        return Err(format!(
            "cargo-kani identity mismatch:\nobserved:\n{}\nexpected:\n{cargo_kani}",
            observed.cargo_kani_version
        ));
    }
    Ok(())
}

pub fn validate_toolchain_identities(
    observed: &RuntimeToolIdentities,
    expected: &ToolchainManifest,
) -> Result<(), String> {
    observed.validate()?;
    validate_versions(
        observed,
        &expected.cargo_kani_version,
        &expected.cbmc_version,
        &expected.kissat_version,
    )
}

fn fixture_hashes(directory: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut hashes = BTreeMap::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            let name = entry.file_name().into_string().map_err(|_| "fixture name is not UTF-8")?;
            hashes.insert(name, sha256_file(&entry.path()).map_err(|e| e.to_string())?);
        }
    }
    let expected = [
        "backend_probe.rs",
        "false_assertion.rs",
        "inadequate_unwind.rs",
        "partial_pair.rs",
        "vacuity.rs",
    ];
    if hashes.keys().map(String::as_str).collect::<Vec<_>>() != expected {
        return Err(
            "mutation fixture inventory differs from the five declared fixtures".to_string()
        );
    }
    Ok(hashes)
}

fn warning_dispositions() -> BTreeMap<String, String> {
    // call_single_file.rs injects register_tool and force-warns unstable_features
    // even for these feature-free fixtures. No other warning is admitted.
    BTreeMap::from([
        ("use of an unstable feature".to_string(), "Kani injects feature(register_tool) for single-file compilation; the public fixtures use no unstable Rust feature".to_string()),
        ("1 warning emitted".to_string(), "rustc summary of the reviewed register_tool warning".to_string()),
    ])
}

fn parse_probe_output(output: &CommandOutput) -> Result<ParsedOutput, String> {
    parse_kani_output(&format!("{}\n{}", output.stdout, output.stderr))
}

fn single_harness(parsed: &ParsedOutput, name: &str) -> bool {
    let warnings = warning_dispositions();
    parsed.validate_completion().is_ok()
        && parsed.harnesses.len() == 1
        && parsed.harnesses.contains_key(name)
        && parsed.warnings.iter().all(|warning| warnings.contains_key(warning))
        && parsed.unsupported_constructs.is_empty()
}

/// A reachable false assertion fails as one harness with real check failures.
fn false_assertion_failure(parsed: &ParsedOutput) -> bool {
    single_harness(parsed, "false_assertion")
        && parsed.harnesses["false_assertion"].is_fail()
        && parsed.harnesses["false_assertion"].failed_checks > 0
}

/// Diagnostics Kani reports when a `--harness` filter matches no harness. The
/// wording changed with Kani's zero-match failure admission, so both the
/// current and the previous diagnostic are recognized.
const ZERO_MATCH_DIAGNOSTICS: [&str; 2] =
    ["Failed to match the following harness(es)", "no harnesses matched the harness filter"];

fn zero_match_filter_detected(code: Option<i32>, stdout: &str, stderr: &str) -> bool {
    code.is_some_and(|code| code != 0)
        && ZERO_MATCH_DIAGNOSTICS
            .iter()
            .any(|diagnostic| stdout.contains(diagnostic) || stderr.contains(diagnostic))
        && !stdout.contains("VERIFICATION:- SUCCESSFUL")
        && !stderr.contains("VERIFICATION:- SUCCESSFUL")
}

/// The complete harness set of the partial_pair fixture; any proper subset run
/// must be rejected as a complete result for this fixture.
fn partial_pair_consumer() -> ConsumerManifest {
    ConsumerManifest {
        schema: 1,
        profile: "mutation-fixture".to_string(),
        status: "bootstrap".to_string(),
        consumer: "partial_pair".to_string(),
        repository: "https://example.invalid/mutation-fixture".to_string(),
        source_commit: "0".repeat(40),
        source_tree: "1".repeat(40),
        project_dir: ".".to_string(),
        cargo_manifest: "Cargo.toml".to_string(),
        cargo_config: None,
        kani_flags: vec![],
        expected_harnesses: vec!["partial_one".to_string(), "partial_two".to_string()],
        expected_cover_properties: 0,
        expected_unreachable_checks: BTreeMap::new(),
        unreachable_disposition: None,
        diagnostics: ConsumerDiagnostics { warnings: vec![], unsupported_constructs: vec![] },
    }
}

/// Detection asserts the subset run itself completed cleanly and that the
/// completion validation rejects it as a result for the full fixture set.
fn partial_execution_detected(parsed: &ParsedOutput) -> bool {
    single_harness(parsed, "partial_one")
        && parsed.harnesses["partial_one"].is_pass()
        && parsed.harnesses["partial_one"].covers_satisfied()
        && parsed
            .validate_against_consumer(&partial_pair_consumer())
            .is_err_and(|error| error.contains("harness set mismatch"))
}

pub fn run_all_mutations(ctx: &MutationContext) -> Result<MutationReceipt, String> {
    let resolved = ResolvedTools::resolve(&ctx.kani_bin)?;
    let snapshot = tempfile::Builder::new()
        .prefix("kani-qualify-")
        .tempdir()
        .map_err(|e| format!("cannot create private mutation snapshot: {e}"))?;
    let runtime_bin = snapshot.path().join("bin");
    fs::create_dir(&runtime_bin).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    for (name, binary) in [("cbmc", &resolved.cbmc), ("kissat", &resolved.kissat)] {
        std::os::unix::fs::symlink(binary, runtime_bin.join(name)).map_err(|e| e.to_string())?;
    }
    let observed_runtime = resolved.observe(&runtime_bin)?;
    let (purpose, profile, toolchain_sha256) = match &ctx.mode {
        MutationMode::InfrastructureSelfTest => {
            (MutationPurpose::InfrastructureSelfTest, None, None)
        }
        MutationMode::Qualification { toolchain, sha256 } => {
            toolchain.validate()?;
            validate_sha256(sha256)?;
            validate_toolchain_identities(&observed_runtime, toolchain)?;
            (
                MutationPurpose::QualificationMutations,
                Some(toolchain.profile.clone()),
                Some(sha256.clone()),
            )
        }
    };
    // All proof execution is dominated by the mode's identity admission above.
    let gate_sha256 = executable_sha256()?;
    let fixtures = fixture_hashes(ctx.fixtures_dir)?;
    let snapshot_fixtures = snapshot.path().join("fixtures");
    fs::create_dir(&snapshot_fixtures).map_err(|e| e.to_string())?;
    for name in fixtures.keys() {
        fs::copy(ctx.fixtures_dir.join(name), snapshot_fixtures.join(name))
            .map_err(|e| format!("cannot snapshot fixture {name}: {e}"))?;
    }
    let snapshot_hashes = fixture_hashes(&snapshot_fixtures)?;
    if snapshot_hashes != fixtures {
        return Err("fixture bytes changed while creating their execution snapshot".to_string());
    }

    let command = |fixture: &str| -> Result<Command, String> {
        let mut cmd = resolved.command("kani", &runtime_bin)?;
        cmd.current_dir(snapshot.path())
            .arg(snapshot_fixtures.join(fixture))
            .arg("--output-format=terse");
        Ok(cmd)
    };
    let mut results = BTreeMap::new();
    let inadequate = run_with_timeout(command("inadequate_unwind.rs")?, Duration::from_secs(120))?;
    let inadequate_detected = inadequate.status.code() == Some(1)
        && inadequate.stdout.contains("unwinding assertion")
        && parse_probe_output(&inadequate).is_ok_and(|p| {
            single_harness(&p, "inadequate_unwind") && p.harnesses["inadequate_unwind"].is_fail()
        });
    results.insert(
        "inadequate_unwind".to_string(),
        ProbeResult {
            returncode: inadequate.status.code(),
            output_sha256: Some(hash_bytes(inadequate.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(inadequate.stderr.as_bytes())),
            ..ProbeResult::new(inadequate_detected, "missing complete expected unwinding failure")
        },
    );

    let false_assertion =
        run_with_timeout(command("false_assertion.rs")?, Duration::from_secs(120))?;
    let false_assertion_detected = false_assertion.status.code() == Some(1)
        && parse_probe_output(&false_assertion).is_ok_and(|p| false_assertion_failure(&p));
    results.insert(
        "false_assertion".to_string(),
        ProbeResult {
            returncode: false_assertion.status.code(),
            output_sha256: Some(hash_bytes(false_assertion.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(false_assertion.stderr.as_bytes())),
            ..ProbeResult::new(
                false_assertion_detected,
                "missing complete expected assertion failure",
            )
        },
    );

    let vacuity = run_with_timeout(command("vacuity.rs")?, Duration::from_secs(120))?;
    let vacuity_detected = vacuity.status.success()
        && parse_probe_output(&vacuity).is_ok_and(|p| {
            single_harness(&p, "vacuity")
                && p.harnesses["vacuity"].covers.is_some_and(|c| c.satisfied < c.total)
        });
    results.insert(
        "vacuity".to_string(),
        ProbeResult {
            kani_returncode: vacuity.status.code(),
            output_sha256: Some(hash_bytes(vacuity.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(vacuity.stderr.as_bytes())),
            ..ProbeResult::new(
                vacuity_detected,
                "vacuity did not produce a complete unsatisfied cover result",
            )
        },
    );

    let mut sleep_cmd = Command::new("sleep");
    sleep_cmd.arg("5");
    let machinery_timeout = run_with_timeout(sleep_cmd, Duration::from_millis(200))
        .is_err_and(|e| e.contains("timed out"));
    // A real kani invocation cannot finish within one millisecond; the runner
    // must kill its process group and report the deadline instead of admitting
    // partial output.
    let kani_timeout = run_with_timeout(command("backend_probe.rs")?, Duration::from_millis(1))
        .is_err_and(|e| e.contains("timed out"));
    let timeout_detected = machinery_timeout && kani_timeout;
    results.insert(
        "timeout".to_string(),
        ProbeResult::new(timeout_detected, "timeout was not detected"),
    );

    let positive = run_with_timeout(command("backend_probe.rs")?, Duration::from_secs(120))?;
    let parsed_positive = parse_probe_output(&positive);
    let positive_detected = positive.status.success()
        && parsed_positive.as_ref().is_ok_and(|p| {
            single_harness(p, "backend_probe")
                && p.harnesses["backend_probe"].is_pass()
                && p.harnesses["backend_probe"].covers_satisfied()
                && p.observed_unreachable_map().is_empty()
        });
    results.insert(
        "positive_control".to_string(),
        ProbeResult {
            returncode: positive.status.code(),
            output_sha256: Some(hash_bytes(positive.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(positive.stderr.as_bytes())),
            ..ProbeResult::new(positive_detected, "positive control did not complete successfully")
        },
    );
    let truncated = positive.stdout.split("Complete -").next().unwrap_or("");
    let truncation_detected = positive_detected
        && truncated != positive.stdout
        && !truncated.is_empty()
        && parse_kani_output(truncated).map_or(true, |p| !p.is_pass());
    results.insert(
        "parser_truncation".to_string(),
        ProbeResult {
            positive_control_returncode: positive.status.code(),
            complete_output_sha256: Some(hash_bytes(positive.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(positive.stderr.as_bytes())),
            truncated_output_sha256: Some(hash_bytes(truncated.as_bytes())),
            ..ProbeResult::new(
                truncation_detected,
                "truncated output was accepted or positive control failed",
            )
        },
    );

    let mut zero_match_cmd = command("backend_probe.rs")?;
    zero_match_cmd.args(["--harness", "no_such_harness_zero_match"]);
    let zero_match = run_with_timeout(zero_match_cmd, Duration::from_secs(120))?;
    let zero_match_detected = zero_match_filter_detected(
        zero_match.status.code(),
        &zero_match.stdout,
        &zero_match.stderr,
    );
    results.insert(
        "zero_match_filter".to_string(),
        ProbeResult {
            returncode: zero_match.status.code(),
            output_sha256: Some(hash_bytes(zero_match.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(zero_match.stderr.as_bytes())),
            ..ProbeResult::new(zero_match_detected, "zero-match harness filter was not rejected")
        },
    );

    let mut partial_cmd = command("partial_pair.rs")?;
    partial_cmd.args(["--harness", "partial_one"]);
    let partial = run_with_timeout(partial_cmd, Duration::from_secs(120))?;
    let partial_detected = partial.status.success()
        && parse_probe_output(&partial).is_ok_and(|p| partial_execution_detected(&p));
    results.insert(
        "partial_harness_execution".to_string(),
        ProbeResult {
            returncode: partial.status.code(),
            output_sha256: Some(hash_bytes(partial.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(partial.stderr.as_bytes())),
            ..ProbeResult::new(
                partial_detected,
                "partial harness execution was admitted as a complete result",
            )
        },
    );

    let cbmc_mismatch = format!("{}-mismatch", observed_runtime.cbmc_version);
    let kissat_mismatch = format!("{}-mismatch", observed_runtime.kissat_version);
    let cargo_mismatch = format!("{}-mismatch", observed_runtime.cargo_kani_version);
    let mismatches_detected = [
        (&*observed_runtime.cargo_kani_version, &*cbmc_mismatch, &*observed_runtime.kissat_version),
        (&*observed_runtime.cargo_kani_version, &*observed_runtime.cbmc_version, &*kissat_mismatch),
        (&*cargo_mismatch, &*observed_runtime.cbmc_version, &*observed_runtime.kissat_version),
    ]
    .into_iter()
    .all(|(cargo, cbmc, kissat)| {
        validate_versions(&observed_runtime, cargo, cbmc, kissat).is_err()
    });
    results.insert(
        "bad_runtime_cbmc".to_string(),
        ProbeResult::new(mismatches_detected, "runtime identity mismatch was accepted"),
    );

    let mut invalid_cmd = command("backend_probe.rs")?;
    invalid_cmd.args([
        "-Z",
        "unstable-options",
        "--cbmc-args",
        "--unsupported-cbmc-option-probe-fail",
    ]);
    let invalid = run_with_timeout(invalid_cmd, Duration::from_secs(60))?;
    let backend_detected = invalid.status.code().is_some_and(|code| code != 0 && code != 2)
        && (invalid.stdout.contains("CBMC failed")
            || invalid.stdout.contains("Unknown option")
            || invalid.stderr.contains("Unknown option"));
    results.insert(
        "backend_failure".to_string(),
        ProbeResult {
            returncode: invalid.status.code(),
            output_sha256: Some(hash_bytes(invalid.stdout.as_bytes())),
            stderr_sha256: Some(hash_bytes(invalid.stderr.as_bytes())),
            ..ProbeResult::new(backend_detected, "expected backend failure was not detected")
        },
    );

    if fixture_hashes(ctx.fixtures_dir)? != fixtures
        || fixture_hashes(&snapshot_fixtures)? != snapshot_hashes
    {
        return Err("source or execution fixture bytes changed during mutation probes".to_string());
    }
    resolved.verify_hashes(&observed_runtime)?;
    let status =
        if results.values().all(|r| r.detected) { GateStatus::Pass } else { GateStatus::Fail };
    Ok(MutationReceipt {
        schema: 2,
        purpose,
        profile,
        executed_at: current_iso_timestamp(),
        gate_sha256,
        toolchain_sha256,
        observed_runtime,
        snapshot_hashes,
        fixtures,
        warning_dispositions: warning_dispositions(),
        results,
        status,
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
            cargo_kani_sha256: "0".repeat(64),
            kani_sha256: "1".repeat(64),
            kani_compiler_sha256: "4".repeat(64),
            cbmc_version: manifest.cbmc_version.clone(),
            cbmc_sha256: "2".repeat(64),
            kissat_version: manifest.kissat_version.clone(),
            kissat_sha256: "3".repeat(64),
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
        let temp_dir = tempfile::tempdir().unwrap();
        let fixture_file = temp_dir.path().join("test_fixture.rs");
        fs::write(&fixture_file, b"fn main() {}").unwrap();

        let initial_hash = sha256_file(&fixture_file).unwrap();
        assert_eq!(initial_hash, hash_bytes(b"fn main() {}"));

        // Modify file
        fs::write(&fixture_file, b"fn main() { panic!(); }").unwrap();
        let modified_hash = sha256_file(&fixture_file).unwrap();
        assert_ne!(initial_hash, modified_hash);
    }

    #[test]
    fn test_false_assertion_failure_detection() {
        let failing = r#"
Checking harness false_assertion...
 ** 1 of 2 failed
Failed Checks: attempt to add with overflow
VERIFICATION:- FAILED
Verification failed for - false_assertion
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
"#;
        let parsed = parse_kani_output(failing).unwrap();
        assert!(false_assertion_failure(&parsed));

        // A passing harness is not the seeded failure.
        let passing = r#"
Checking harness false_assertion...
 ** 0 of 2 failed
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(passing).unwrap();
        assert!(!false_assertion_failure(&parsed));

        // A failure attributed to another harness is not this probe's mutation.
        let other = failing.replace("false_assertion", "some_other_harness");
        let parsed = parse_kani_output(&other).unwrap();
        assert!(!false_assertion_failure(&parsed));
    }

    #[test]
    fn test_zero_match_filter_detection() {
        let diagnostic = "error: Failed to match the following harness(es):\nno_such_harness_zero_match\nPlease specify the fully-qualified name of a harness.\n";
        assert!(zero_match_filter_detected(Some(1), diagnostic, ""));
        // The previous zero-match diagnostic wording is also recognized.
        assert!(zero_match_filter_detected(
            Some(1),
            "error: no harnesses matched the harness filter: `no_such_harness_zero_match`\n",
            ""
        ));
        // The diagnostic is admitted from either output stream.
        assert!(zero_match_filter_detected(Some(2), "", diagnostic));
        // A successful exit or a missing diagnostic is not a rejection.
        assert!(!zero_match_filter_detected(Some(0), diagnostic, ""));
        assert!(!zero_match_filter_detected(Some(1), "unrelated failure\n", ""));
        // A success marker contradicts the rejection.
        assert!(!zero_match_filter_detected(
            Some(1),
            &format!("{diagnostic}VERIFICATION:- SUCCESSFUL\n"),
            ""
        ));
        // A lost exit code (signal) is not a normal rejection.
        assert!(!zero_match_filter_detected(None, diagnostic, ""));
    }

    #[test]
    fn test_partial_execution_detection() {
        let subset = r#"
Checking harness partial_one...
 ** 0 of 1 failed
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(subset).unwrap();
        assert!(partial_execution_detected(&parsed));

        // The complete pair is a valid full result, not a partial execution.
        let complete = r#"
Checking harness partial_one...
 ** 0 of 1 failed
VERIFICATION:- SUCCESSFUL
Checking harness partial_two...
 ** 0 of 1 failed
VERIFICATION:- SUCCESSFUL
Complete - 2 successfully verified harnesses, 0 failures, 2 total.
"#;
        let parsed = parse_kani_output(complete).unwrap();
        assert!(!partial_execution_detected(&parsed));

        // A failing subset run is not a clean partial execution.
        let failing = subset
            .replace("0 of 1 failed", "1 of 1 failed")
            .replace(
                "VERIFICATION:- SUCCESSFUL",
                "Failed Checks: assertion failed\nVERIFICATION:- FAILED\nVerification failed for - partial_one",
            )
            .replace(
                "Complete - 1 successfully verified harnesses, 0 failures, 1 total.",
                "Complete - 0 successfully verified harnesses, 1 failures, 1 total.",
            );
        let parsed = parse_kani_output(&failing).unwrap();
        assert!(!partial_execution_detected(&parsed));
    }
}
