// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::gate::{current_iso_timestamp, sha256_file};
use crate::model::{
    CompositeReceipt, ConsumerManifest, CoverMetrics, GateStatus, HarnessSummary, InputRunEvidence,
    ToolchainManifest, VerificationSummary,
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
        if !seen_run_ids.insert(&run.run_id) {
            return Err(format!("duplicate run_id detected in evidence: {}", run.run_id));
        }

        let path = Path::new(&run.log_path);
        let canonical = fs::canonicalize(path)
            .map_err(|e| format!("cannot canonicalize log path {}: {e}", run.log_path))?;
        if !seen_canonical_paths.insert(canonical) {
            return Err(format!("aliased or duplicate log path detected: {}", run.log_path));
        }

        if run.exit_code != 0 || run.terminating_signal.is_some() {
            return Err(format!(
                "run {} produced non-zero exit code ({}) or terminating signal ({:?})",
                run.run_id, run.exit_code, run.terminating_signal
            ));
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

        // Fail-closed check: individual log completion summary validation
        let summary = parsed.summary.ok_or_else(|| {
            format!("log {} is truncated or missing completion summary", run.log_path)
        })?;

        if summary.total != parsed.harnesses.len() as u32 {
            return Err(format!(
                "log {} summary total ({}) does not match parsed harness count ({})",
                run.log_path,
                summary.total,
                parsed.harnesses.len()
            ));
        }

        let parsed_pass = parsed.harnesses.values().filter(|h| h.is_pass()).count() as u32;
        let parsed_fail = parsed.harnesses.values().filter(|h| h.is_fail()).count() as u32;

        if summary.successful != parsed_pass || summary.failed != parsed_fail {
            return Err(format!(
                "log {} summary counts (pass: {}, fail: {}) contradict parsed verdicts (pass: {}, fail: {})",
                run.log_path, summary.successful, summary.failed, parsed_pass, parsed_fail
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
            *all_unsupported.entry(construct).or_insert(0) += count;
        }
    }

    all_warnings.sort();
    all_warnings.dedup();

    let total = all_harnesses.len() as u32;
    let successful = all_harnesses.values().filter(|h| h.is_pass()).count() as u32;
    let failed = all_harnesses.values().filter(|h| h.is_fail()).count() as u32;

    let composite_output = ParsedOutput {
        harnesses: all_harnesses.clone(),
        warnings: all_warnings.clone(),
        unsupported_constructs: all_unsupported.clone(),
        summary: Some(VerificationSummary { successful, failed, total }),
    };

    let status = match composite_output.validate_against_consumer(consumer) {
        Ok(()) => GateStatus::Pass,
        Err(e) => {
            eprintln!("Qualification gate check failed: {e}");
            GateStatus::Fail
        }
    };

    let total_satisfied = all_harnesses.values().map(|h| h.covers.map_or(0, |c| c.satisfied)).sum();
    let total_covers = all_harnesses.values().map(|h| h.covers.map_or(0, |c| c.total)).sum();

    let gate_sha256 = match std::env::current_exe() {
        Ok(exe) => sha256_file(&exe).unwrap_or_else(|_| hash_bytes(b"kani-qualify-v0.1.0")),
        Err(_) => hash_bytes(b"kani-qualify-v0.1.0"),
    };

    let mut receipt = CompositeReceipt {
        schema: 1,
        profile: consumer.profile.clone(),
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
        harnesses: all_harnesses,
        input_runs: input_runs.to_vec(),
        cover_properties: CoverMetrics { satisfied: total_satisfied, total: total_covers },
        unreachable_checks: composite_output.observed_unreachable_map(),
        warnings: all_warnings,
        unsupported_constructs: all_unsupported,
        receipt_sha256: None,
    };

    receipt.receipt_sha256 = Some(receipt.compute_canonical_sha256()?);

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::path::PathBuf;

    struct TempTestDir(PathBuf);
    impl TempTestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "kani_qualify_test_{}_{}_{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = fs::create_dir_all(&path);
            Self(path)
        }
    }
    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

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

    #[test]
    fn test_empty_runs_fail() {
        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new());
        let res = compose_runs(&toolchain, &consumer, "sha_tc", "sha_c", &[]);
        assert!(res.is_err());
    }

    #[test]
    fn test_same_basename_different_dirs_and_ids() {
        let temp1 = TempTestDir::new("dir1");
        let temp2 = TempTestDir::new("dir2");
        let log1_path = temp1.0.join("run.log");
        let log2_path = temp2.0.join("run.log");

        let log1_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
        let log2_content = "Checking harness h2...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";

        fs::write(&log1_path, log1_content).unwrap();
        fs::write(&log2_path, log2_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1", "h2"], BTreeMap::new());

        let runs = vec![
            InputRunEvidence {
                run_id: "run-1".to_string(),
                log_path: log1_path.to_string_lossy().to_string(),
                sha256: hash_bytes(log1_content.as_bytes()),
                argv: vec!["kani".to_string(), "--harness".to_string(), "h1".to_string()],
                exit_code: 0,
                terminating_signal: None,
            },
            InputRunEvidence {
                run_id: "run-2".to_string(),
                log_path: log2_path.to_string_lossy().to_string(),
                sha256: hash_bytes(log2_content.as_bytes()),
                argv: vec!["kani".to_string(), "--harness".to_string(), "h2".to_string()],
                exit_code: 0,
                terminating_signal: None,
            },
        ];

        let receipt = compose_runs(&toolchain, &consumer, "tc_sha", "c_sha", &runs).unwrap();
        assert_eq!(receipt.status, GateStatus::Pass);
        assert_eq!(receipt.total_harnesses, 2);
        assert_eq!(receipt.input_runs.len(), 2);
        assert!(receipt.receipt_sha256.is_some());
    }

    #[test]
    fn test_nonzero_producer_fails_closed() {
        let temp = TempTestDir::new("nonzero");
        let log_path = temp.0.join("run.log");
        let log_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
        fs::write(&log_path, log_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new());

        let runs = vec![InputRunEvidence {
            run_id: "run-1".to_string(),
            log_path: log_path.to_string_lossy().to_string(),
            sha256: hash_bytes(log_content.as_bytes()),
            argv: vec!["kani".to_string()],
            exit_code: 1, // Non-zero exit code
            terminating_signal: None,
        }];

        let res = compose_runs(&toolchain, &consumer, "tc_sha", "c_sha", &runs);
        assert!(res.is_err());
    }

    #[test]
    fn test_missing_harness_set_fails_gate() {
        let temp = TempTestDir::new("missing_harness");
        let log_path = temp.0.join("run.log");
        let log_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
        fs::write(&log_path, log_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1", "h2"], BTreeMap::new()); // expects h1 and h2, but log only has h1

        let runs = vec![InputRunEvidence {
            run_id: "run-1".to_string(),
            log_path: log_path.to_string_lossy().to_string(),
            sha256: hash_bytes(log_content.as_bytes()),
            argv: vec!["kani".to_string()],
            exit_code: 0,
            terminating_signal: None,
        }];

        let receipt = compose_runs(&toolchain, &consumer, "tc_sha", "c_sha", &runs).unwrap();
        assert_eq!(receipt.status, GateStatus::Fail);
    }

    #[test]
    fn test_extra_unexpected_harness_fails_gate() {
        let temp = TempTestDir::new("extra_harness");
        let log_path = temp.0.join("run.log");
        let log_content = "Checking harness h1...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nChecking harness h_extra...\n ** 0 of 1 failed\nVERIFICATION:- SUCCESSFUL\nComplete - 2 successfully verified harnesses, 0 failures, 2 total.\n";
        fs::write(&log_path, log_content).unwrap();

        let toolchain = sample_toolchain();
        let consumer = sample_consumer(vec!["h1"], BTreeMap::new()); // expects only h1, but log has h_extra

        let runs = vec![InputRunEvidence {
            run_id: "run-1".to_string(),
            log_path: log_path.to_string_lossy().to_string(),
            sha256: hash_bytes(log_content.as_bytes()),
            argv: vec!["kani".to_string()],
            exit_code: 0,
            terminating_signal: None,
        }];

        let receipt = compose_runs(&toolchain, &consumer, "tc_sha", "c_sha", &runs).unwrap();
        assert_eq!(receipt.status, GateStatus::Fail);
    }
}
