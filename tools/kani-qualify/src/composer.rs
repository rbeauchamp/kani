// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::gate::{current_iso_timestamp, executable_sha256};
use crate::model::{
    CompositeReceipt, ConsumerManifest, GateStatus, HarnessSummary, InputRunEvidence,
    ToolchainManifest, VerificationSummary, validate_sha256,
};
use crate::mutations::hash_bytes;
use crate::parser::{ParsedOutput, parse_kani_output};

pub fn compose_runs(
    toolchain: &ToolchainManifest,
    consumer: &ConsumerManifest,
    toolchain_sha256: &str,
    consumer_sha256: &str,
    input_runs: &[InputRunEvidence],
) -> Result<CompositeReceipt, String> {
    toolchain.validate()?;
    consumer.validate()?;
    validate_sha256(toolchain_sha256)?;
    validate_sha256(consumer_sha256)?;

    if toolchain.profile != consumer.profile {
        return Err(format!(
            "toolchain and consumer profiles differ: {} vs {}",
            toolchain.profile, consumer.profile
        ));
    }

    if input_runs.is_empty() {
        return Err(
            "cannot compose empty input runs: at least one run evidence required".to_string()
        );
    }

    // Validate unique run IDs and distinct, unaliased canonical log paths
    let mut seen_run_ids = BTreeSet::new();
    let mut seen_canonical_paths = BTreeSet::new();

    for run in input_runs {
        run.validate(toolchain, consumer, toolchain_sha256, consumer_sha256)?;
        if run.context != input_runs[0].context {
            return Err(
                "cannot combine different runtime, platform, or qualification contexts".to_string()
            );
        }
        if !seen_run_ids.insert(&run.run_id) {
            return Err(format!("duplicate run_id detected in evidence: {}", run.run_id));
        }

        let path = Path::new(&run.log_path);
        let canonical = fs::canonicalize(path)
            .map_err(|e| format!("cannot canonicalize log path {}: {e}", run.log_path))?;
        if !seen_canonical_paths.insert(canonical) {
            return Err(format!("aliased or duplicate log path detected: {}", run.log_path));
        }
    }

    let mut all_harnesses: BTreeMap<String, HarnessSummary> = BTreeMap::new();
    let mut all_warnings = Vec::new();
    let mut all_unsupported = BTreeMap::new();

    for run in input_runs {
        let raw_bytes = fs::read(&run.log_path)
            .map_err(|e| format!("failed to read log {}: {e}", run.log_path))?;

        let actual_hash = hash_bytes(&raw_bytes);
        if actual_hash != run.sha256 {
            return Err(format!(
                "log {} digest mismatch: evidence claimed {}, but bytes hashed to {}",
                run.log_path, run.sha256, actual_hash
            ));
        }

        let content = String::from_utf8(raw_bytes)
            .map_err(|e| format!("log {} contains invalid UTF-8: {e}", run.log_path))?;

        let parsed = parse_kani_output(&content)?;
        parsed.validate_completion()?;
        let observed: BTreeSet<_> = parsed.harnesses.keys().collect();
        let selected: BTreeSet<_> = run.expected_harnesses.iter().collect();
        if observed != selected {
            return Err(format!(
                "run {} did not report exactly its selected harnesses",
                run.run_id
            ));
        }

        for (harness, summary) in parsed.harnesses {
            match all_harnesses.entry(harness) {
                Entry::Occupied(existing) => {
                    if existing.get() != &summary {
                        return Err(format!(
                            "conflicting verification records for {} across composed logs:\n{:?}\nvs\n{:?}",
                            existing.key(),
                            existing.get(),
                            summary
                        ));
                    }
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(summary);
                }
            }
        }

        all_warnings.extend(parsed.warnings);
        for (construct, count) in parsed.unsupported_constructs {
            let total = all_unsupported.entry(construct).or_insert(0u32);
            *total = total.checked_add(count).ok_or("unsupported-construct total overflow")?;
        }
    }

    all_warnings.sort();
    all_warnings.dedup();

    let total = u32::try_from(all_harnesses.len()).map_err(|e| e.to_string())?;
    let successful = u32::try_from(all_harnesses.values().filter(|h| h.is_pass()).count())
        .map_err(|e| e.to_string())?;
    let failed = total - successful;

    let composite_output = ParsedOutput {
        harnesses: all_harnesses,
        warnings: all_warnings,
        unsupported_constructs: all_unsupported,
        summary: Some(VerificationSummary { successful, failed, total }),
    };

    let status = match composite_output.validate_against_consumer(consumer) {
        Ok(()) => GateStatus::Pass,
        Err(e) => {
            eprintln!("Qualification gate check failed: {e}");
            GateStatus::Fail
        }
    };

    let cover_properties = composite_output.cover_totals()?;
    let unreachable_checks = composite_output.observed_unreachable_map();
    let gate_sha256 = executable_sha256()?;

    let mut receipt = CompositeReceipt {
        schema: 2,
        profile: consumer.profile.clone(),
        consumer_status: consumer.status.clone(),
        executed_at: current_iso_timestamp(),
        gate_sha256,
        toolchain_sha256: toolchain_sha256.to_string(),
        consumer_sha256: consumer_sha256.to_string(),
        source_commit: consumer.source_commit.clone(),
        source_tree: consumer.source_tree.clone(),
        status,
        total_harnesses: total,
        successful_harnesses: successful,
        failed_harnesses: failed,
        harnesses: composite_output.harnesses,
        input_runs: input_runs.to_vec(),
        cover_properties,
        unreachable_checks,
        warnings: composite_output.warnings,
        unsupported_constructs: composite_output.unsupported_constructs,
        receipt_sha256: None,
    };

    receipt.receipt_sha256 = Some(receipt.compute_canonical_sha256()?);

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn sample_toolchain() -> ToolchainManifest {
        ToolchainManifest {
            schema: 1,
            profile: "core-v1".to_string(),
            kani_base: "0".repeat(40),
            kani_tree: "1".repeat(40),
            cargo_kani_version: "0.67.0".to_string(),
            cbmc_version: "6.10.0".to_string(),
            kissat_version: "4.0.1".to_string(),
            platforms: {
                let mut map = BTreeMap::new();
                map.insert(
                    "x86_64-unknown-linux-gnu".to_string(),
                    PlatformManifest {
                        runner: "ubuntu-22.04".to_string(),
                        artifacts: vec![ArtifactEntry {
                            name: "kani.tar.gz".to_string(),
                            sha256: "0".repeat(64),
                        }],
                    },
                );
                map
            },
        }
    }

    fn sample_consumer(
        harnesses: Vec<&str>,
        unreachable: BTreeMap<String, u32>,
    ) -> ConsumerManifest {
        ConsumerManifest {
            schema: 1,
            profile: "core-v1".to_string(),
            status: "bootstrap".to_string(),
            consumer: "test-pkg".to_string(),
            repository: "https://example.invalid/test".to_string(),
            source_commit: "0".repeat(40),
            source_tree: "1".repeat(40),
            project_dir: ".".to_string(),
            cargo_manifest: "Cargo.toml".to_string(),
            cargo_config: None,
            kani_flags: vec![],
            expected_harnesses: harnesses.into_iter().map(|s| s.to_string()).collect(),
            expected_cover_properties: 0,
            expected_unreachable_checks: unreachable,
            unreachable_disposition: None,
            diagnostics: ConsumerDiagnostics { warnings: vec![], unsupported_constructs: vec![] },
        }
    }

    fn manifest_hash<T: serde::Serialize>(manifest: &T) -> String {
        hash_bytes(&serde_json::to_vec(manifest).unwrap())
    }

    fn evidence(
        consumer: &ConsumerManifest,
        path: &Path,
        id: &str,
        selected: &[&str],
    ) -> InputRunEvidence {
        let toolchain = sample_toolchain();
        let cwd = path.parent().unwrap().to_string_lossy().into_owned();
        let mut argv = vec![
            "cargo".to_string(),
            "kani".to_string(),
            "--manifest-path".to_string(),
            Path::new(&cwd).join(&consumer.cargo_manifest).to_string_lossy().into_owned(),
            "--exact".to_string(),
            "--output-format=terse".to_string(),
        ];
        for name in selected {
            argv.extend(["--harness".to_string(), (*name).to_string()]);
        }
        InputRunEvidence {
            context: RunContext {
                consumer_sha256: manifest_hash(consumer),
                platform: "x86_64-unknown-linux-gnu".to_string(),
                runtime: RuntimeToolIdentities {
                    cargo_kani_version: toolchain.cargo_kani_version.clone(),
                    cargo_kani_sha256: "0".repeat(64),
                    kani_sha256: "0".repeat(64),
                    kani_compiler_sha256: "1".repeat(64),
                    cbmc_version: toolchain.cbmc_version.clone(),
                    cbmc_sha256: "2".repeat(64),
                    kissat_version: toolchain.kissat_version.clone(),
                    kissat_sha256: "3".repeat(64),
                },
                source_commit: consumer.source_commit.clone(),
                source_tree: consumer.source_tree.clone(),
                toolchain_sha256: manifest_hash(&toolchain),
            },
            cwd,
            expected_harnesses: selected.iter().map(|s| (*s).to_string()).collect(),
            run_id: id.to_string(),
            log_path: path.to_string_lossy().into_owned(),
            sha256: hash_bytes(&fs::read(path).unwrap()),
            argv,
            exit_code: 0,
            terminating_signal: None,
        }
    }

    fn compose_test_runs(
        toolchain: &ToolchainManifest,
        consumer: &ConsumerManifest,
        runs: &[InputRunEvidence],
    ) -> Result<CompositeReceipt, String> {
        compose_runs(toolchain, consumer, &manifest_hash(toolchain), &manifest_hash(consumer), runs)
    }

    #[test]
    fn test_empty_runs_fail() {
        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new());
        let res = compose_test_runs(&toolchain, &consumer, &[]);
        assert!(res.is_err());
    }

    #[test]
    fn test_profile_mismatch_fails() {
        let mut toolchain = sample_toolchain();
        toolchain.profile = "core-v2".to_string();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new());
        let err = compose_test_runs(&toolchain, &consumer, &[]).unwrap_err();
        assert!(err.contains("toolchain and consumer profiles differ"));
    }

    #[test]
    fn test_same_basename_different_dirs_and_ids() {
        let temp1 = tempfile::tempdir().unwrap();
        let temp2 = tempfile::tempdir().unwrap();
        let log1_path = temp1.path().join("run.log");
        let log2_path = temp2.path().join("run.log");

        let log1_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
        let log2_content = "Checking harness h2...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";

        fs::write(&log1_path, log1_content).unwrap();
        fs::write(&log2_path, log2_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1", "h2"], BTreeMap::new());

        let runs = vec![
            evidence(&consumer, &log1_path, "run-1", &["h1"]),
            evidence(&consumer, &log2_path, "run-2", &["h2"]),
        ];

        let receipt = compose_test_runs(&toolchain, &consumer, &runs).unwrap();
        assert_eq!(receipt.status, GateStatus::Pass);
        assert_eq!(receipt.total_harnesses, 2);
        assert_eq!(receipt.input_runs.len(), 2);
        assert!(receipt.receipt_sha256.is_some());
    }

    #[test]
    fn test_nonzero_producer_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("run.log");
        let log_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
        fs::write(&log_path, log_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new());

        let runs = vec![InputRunEvidence {
            exit_code: 1,
            ..evidence(&consumer, &log_path, "run-1", &["h1"])
        }];

        let res = compose_test_runs(&toolchain, &consumer, &runs);
        assert!(res.is_err());
    }

    #[test]
    fn test_missing_harness_set_fails_gate() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("run.log");
        let log_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
        fs::write(&log_path, log_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1", "h2"], BTreeMap::new()); // expects h1 and h2, but log only has h1

        let runs = vec![evidence(&consumer, &log_path, "run-1", &["h1"])];

        let receipt = compose_test_runs(&toolchain, &consumer, &runs).unwrap();
        assert_eq!(receipt.status, GateStatus::Fail);
    }

    #[test]
    fn test_extra_unexpected_harness_fails_gate() {
        let temp = tempfile::tempdir().unwrap();
        let log_path = temp.path().join("run.log");
        let log_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nChecking harness h_extra...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 2 successfully verified harnesses, 0 failures, 2 total.\n";
        fs::write(&log_path, log_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new()); // expects only h1, but log has h_extra

        let runs = vec![evidence(&consumer, &log_path, "run-1", &["h1"])];
        assert!(compose_test_runs(&toolchain, &consumer, &runs).is_err());
    }
}
