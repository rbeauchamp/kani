// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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

        for (harness, summary) in parsed.harnesses {
            if let Some(existing) = all_harnesses.get(&harness) {
                // If previously failing and now passing, accept the passing retry if explicit
                if existing.successful && !summary.successful {
                    return Err(format!("conflicting verification verdicts for {harness}"));
                }
            }
            all_harnesses.insert(harness, summary);
        }

        all_warnings.extend(parsed.warnings);
        for (construct, count) in parsed.unsupported_constructs {
            *all_unsupported.entry(construct).or_insert(0) += count;
        }
    }

    all_warnings.sort();
    all_warnings.dedup();

    let total = all_harnesses.len() as u32;
    let successful = all_harnesses.values().filter(|h| h.successful).count() as u32;
    let failed = all_harnesses.values().filter(|h| h.failed).count() as u32;
    let status = if failed == 0 && successful == total && total > 0 {
        GateStatus::Pass
    } else {
        GateStatus::Fail
    };

    Ok(CompositeReceipt {
        schema: 1,
        executed_at: format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ),
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

    #[test]
    fn test_empty_logs() {
        let receipt = compose_logs(&[]).unwrap();
        assert_eq!(receipt.total_harnesses, 0);
        assert_eq!(receipt.status, GateStatus::Fail); // Empty run fails closed
    }
}
