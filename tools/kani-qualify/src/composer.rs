// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs;
use std::path::Path;

use crate::gate::current_iso_timestamp;
use crate::model::{CompositeReceipt, GateStatus, HarnessSummary};
use crate::parser::parse_kani_output;

pub fn compose_logs(log_paths: &[&Path]) -> Result<CompositeReceipt, String> {
    let mut all_harnesses: BTreeMap<String, HarnessSummary> = BTreeMap::new();
    let mut all_warnings = Vec::new();
    let mut all_unsupported = BTreeMap::new();

    for log_path in log_paths {
        let content = fs::read_to_string(log_path)
            .map_err(|e| format!("failed to read log {}: {e}", log_path.display()))?;
        let parsed = parse_kani_output(&content)?;

        // Fail-closed check: every log must have a valid summary matching parsed count
        let summary = parsed.summary.ok_or_else(|| {
            format!("log {} is truncated or missing completion summary", log_path.display())
        })?;

        if summary.total != parsed.harnesses.len() as u32 {
            return Err(format!(
                "log {} summary total ({}) does not match parsed harness count ({})",
                log_path.display(),
                summary.total,
                parsed.harnesses.len()
            ));
        }

        for (harness, summary) in parsed.harnesses {
            match all_harnesses.entry(harness) {
                Entry::Occupied(existing) => {
                    if existing.get().verdict != summary.verdict {
                        return Err(format!(
                            "conflicting verification verdicts for {} across composed logs: {:?} vs {:?}",
                            existing.key(),
                            existing.get().verdict,
                            summary.verdict
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
    let mut successful = 0;
    let mut failed = 0;
    let mut all_covers_satisfied = true;

    for h in all_harnesses.values() {
        if h.is_pass() {
            successful += 1;
        } else {
            failed += 1;
        }
        if !h.covers.is_none_or(|c| c.satisfied == c.total) {
            all_covers_satisfied = false;
        }
    }

    let status = if failed == 0 && successful == total && total > 0 && all_covers_satisfied {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    Ok(CompositeReceipt {
        schema: 1,
        executed_at: current_iso_timestamp(),
        status,
        total_harnesses: total,
        successful_harnesses: successful,
        failed_harnesses: failed,
        harnesses: all_harnesses,
        warnings: all_warnings,
        unsupported_constructs: all_unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempTestDir(PathBuf);
    impl TempTestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "kani_qualify_test_{}_{}",
                name,
                std::process::id()
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

    #[test]
    fn test_empty_logs() {
        let receipt = compose_logs(&[]).unwrap();
        assert_eq!(receipt.total_harnesses, 0);
        assert_eq!(receipt.status, GateStatus::Fail); // Empty run fails closed
    }

    #[test]
    fn test_conflicting_verdicts_fail_closed() {
        let temp = TempTestDir::new("conflicting");
        let log1_path = temp.0.join("log1.log");
        let log2_path = temp.0.join("log2.log");

        let log1_content = "Checking harness test::h...\nVERIFICATION:- FAILED\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.\n";
        let log2_content = "Checking harness test::h...\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";

        fs::write(&log1_path, log1_content).unwrap();
        fs::write(&log2_path, log2_content).unwrap();

        let err1 = compose_logs(&[&log1_path, &log2_path]).unwrap_err();
        assert!(err1.contains("conflicting verification verdicts"));

        let err2 = compose_logs(&[&log2_path, &log1_path]).unwrap_err();
        assert!(err2.contains("conflicting verification verdicts"));
    }

    #[test]
    fn test_truncated_log_without_summary_fails_closed() {
        let temp = TempTestDir::new("truncated");
        let log_path = temp.0.join("truncated.log");

        let log_content = "Checking harness test::h...\nVERIFICATION:- SUCCESSFUL\n";
        fs::write(&log_path, log_content).unwrap();

        let err = compose_logs(&[&log_path]).unwrap_err();
        assert!(err.contains("truncated or missing completion summary"));
    }
}
