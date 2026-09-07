// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use crate::model::{
    ConsumerManifest, CoverMetrics, HarnessSummary, HarnessVerdict, VerificationSummary,
};

static CHECK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Checking harness ([a-zA-Z0-9_:]+)\.\.\.$").unwrap());
static FAILED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\*\* ([0-9]+) of ([0-9]+) failed(?: \((?:([0-9]+) undetermined(?:,([0-9]+) unreachable)?|([0-9]+) unreachable)\))?$").unwrap()
});
static COVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\*\* ([0-9]+) of ([0-9]+) cover properties satisfied(?: \((?:([0-9]+) undetermined(?:,([0-9]+) unreachable)?|([0-9]+) unreachable)\))?$",
    )
    .unwrap()
});
static UNSUPPORTED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- (.+) \(([0-9]+)\)$").unwrap());
static SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^Complete - ([0-9]+) successfully verified (?:functions|harnesses), ([0-9]+) failures, ([0-9]+) total\.$",
    )
    .unwrap()
});
const UNSUPPORTED_HEADER: &str = "warning: Found the following unsupported constructs:";

#[derive(Debug, Clone)]
pub struct ParsedOutput {
    pub harnesses: BTreeMap<String, HarnessSummary>,
    pub warnings: Vec<String>,
    pub unsupported_constructs: BTreeMap<String, u32>,
    pub summary: Option<VerificationSummary>,
}

impl ParsedOutput {
    pub fn observed_unreachable_map(&self) -> BTreeMap<String, u32> {
        self.harnesses
            .iter()
            .filter_map(|(name, h)| {
                h.unreachable.filter(|count| *count > 0).map(|count| (name.clone(), count))
            })
            .collect()
    }

    /// Counts must describe the same complete set of parsed terminal records.
    pub fn validate_completion(&self) -> Result<(), String> {
        let summary = self.summary.ok_or("missing completion summary")?;
        let successful = self.harnesses.values().filter(|h| h.is_pass()).count();
        let failed = self.harnesses.values().filter(|h| h.is_fail()).count();
        if self.harnesses.is_empty()
            || usize::try_from(summary.total) != Ok(self.harnesses.len())
            || usize::try_from(summary.successful) != Ok(successful)
            || usize::try_from(summary.failed) != Ok(failed)
        {
            return Err("completion summary contradicts the terminal harness records".to_string());
        }
        Ok(())
    }

    pub fn cover_totals(&self) -> Result<CoverMetrics, String> {
        self.harnesses.values().try_fold(CoverMetrics::default(), |sum, h| {
            let covers = h.covers.unwrap_or_default();
            Ok(CoverMetrics {
                satisfied: sum
                    .satisfied
                    .checked_add(covers.satisfied)
                    .ok_or("cover satisfaction total overflow")?,
                total: sum
                    .total
                    .checked_add(covers.total)
                    .ok_or("cover property total overflow")?,
            })
        })
    }

    /// A self-test pass requires complete successful results and no diagnostics.
    pub fn is_pass(&self) -> bool {
        self.validate_completion().is_ok()
            && self.harnesses.values().all(|h| h.is_pass() && h.covers_satisfied())
            && self.warnings.is_empty()
            && self.unsupported_constructs.is_empty()
    }

    pub fn validate_against_consumer(&self, consumer: &ConsumerManifest) -> Result<(), String> {
        consumer.validate()?;
        self.validate_completion()?;
        let observed: BTreeSet<_> = self.harnesses.keys().collect();
        let expected: BTreeSet<_> = consumer.expected_harnesses.iter().collect();
        if observed != expected {
            return Err(format!(
                "harness set mismatch:\nobserved: {observed:?}\nexpected: {expected:?}"
            ));
        }
        for (name, h) in &self.harnesses {
            if !h.is_pass() || !h.covers_satisfied() {
                return Err(format!("harness {name} failed verification or a cover obligation"));
            }
        }
        let unreachable = self.observed_unreachable_map();
        if unreachable != consumer.expected_unreachable_checks {
            return Err(format!(
                "unreachable-check distribution changed:\nobserved: {unreachable:?}\nexpected: {:?}",
                consumer.expected_unreachable_checks
            ));
        }
        let covers = self.cover_totals()?;
        if covers.satisfied != covers.total || covers.total != consumer.expected_cover_properties {
            return Err(format!(
                "cover obligations changed or failed: observed {}/{}, expected {}",
                covers.satisfied, covers.total, consumer.expected_cover_properties
            ));
        }
        let expected_warnings: BTreeSet<_> =
            consumer.diagnostics.warnings.iter().map(|w| &w.message).collect();
        let observed_warnings: BTreeSet<_> = self.warnings.iter().collect();
        if observed_warnings != expected_warnings {
            return Err(format!(
                "warning ledger mismatch:\nobserved: {observed_warnings:?}\nexpected: {expected_warnings:?}"
            ));
        }
        let expected_unsupported: BTreeSet<_> =
            consumer.diagnostics.unsupported_constructs.iter().map(|c| &c.construct).collect();
        let observed_unsupported: BTreeSet<_> = self.unsupported_constructs.keys().collect();
        if observed_unsupported != expected_unsupported {
            return Err(format!(
                "unsupported-construct ledger mismatch:\nobserved: {observed_unsupported:?}\nexpected: {expected_unsupported:?}"
            ));
        }
        Ok(())
    }
}

struct PendingHarness {
    name: String,
    checks: Option<(u32, u32, u32, Option<u32>)>,
    covers: Option<CoverMetrics>,
    failure_details: u32,
    undetermined_covers: u32,
    unreachable_covers: u32,
    verdict: Option<HarnessVerdict>,
}

impl PendingHarness {
    fn finish(self) -> Result<HarnessSummary, String> {
        let (failed_checks, total_checks, undetermined_checks, unreachable) = self
            .checks
            .ok_or_else(|| format!("missing failure record for harness {}", self.name))?;
        let verdict = self
            .verdict
            .ok_or_else(|| format!("missing verification verdict for harness {}", self.name))?;
        if (verdict == HarnessVerdict::Pass) != (failed_checks == 0 && undetermined_checks == 0) {
            return Err(format!(
                "contradictory failure count and verdict for harness {}",
                self.name
            ));
        }
        if self.failure_details != failed_checks {
            return Err(format!(
                "failure details differ from check count for harness {}",
                self.name
            ));
        }
        Ok(HarnessSummary {
            harness: self.name,
            verdict,
            failed_checks,
            total_checks,
            undetermined_checks,
            undetermined_covers: self.undetermined_covers,
            unreachable,
            covers: self.covers,
            unreachable_covers: self.unreachable_covers,
        })
    }
}

fn finish_harness(
    pending: &mut Option<PendingHarness>,
    harnesses: &mut BTreeMap<String, HarnessSummary>,
) -> Result<(), String> {
    if let Some(pending) = pending.take() {
        let harness = pending.finish()?;
        let name = harness.harness.clone();
        if harnesses.insert(name.clone(), harness).is_some() {
            return Err(format!("duplicate harness evidence found in log for: {name}"));
        }
    }
    Ok(())
}

fn count(text: &str) -> Result<u32, String> {
    text.parse().map_err(|e| format!("invalid result count {text:?}: {e}"))
}

/// Kani's renderer partitions each property inventory into disjoint statuses.
fn validate_counts(
    selected: u32,
    undetermined: u32,
    unreachable: u32,
    total: u32,
) -> Result<(), String> {
    if !crate::counts::counts_fit(selected, undetermined, unreachable, total) {
        return Err("result counts exceed the property inventory".to_string());
    }
    Ok(())
}

/// Consume every result-bearing line exactly once. Progress text is not a result;
/// malformed, duplicate, out-of-order, and unknown result records are errors.
pub fn parse_kani_output(text: &str) -> Result<ParsedOutput, String> {
    let mut output = ParsedOutput {
        harnesses: BTreeMap::new(),
        warnings: Vec::new(),
        unsupported_constructs: BTreeMap::new(),
        summary: None,
    };
    let mut pending: Option<PendingHarness> = None;
    let mut unsupported_entries: Option<usize> = None;
    let mut reported_failures = BTreeSet::new();
    let mut summary_started = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if let Some(entries) = unsupported_entries.as_mut() {
            if line.is_empty() {
                if *entries == 0 {
                    return Err("empty unsupported-construct diagnostic".to_string());
                }
                unsupported_entries = None;
                continue;
            }
            let record = UNSUPPORTED_RE
                .captures(line)
                .ok_or_else(|| format!("unparsed unsupported-construct diagnostic: {line}"))?;
            let name = record[1].trim().to_string();
            let n = count(&record[2])?;
            if name.is_empty() || n == 0 {
                return Err("invalid unsupported-construct diagnostic".to_string());
            }
            let total = output.unsupported_constructs.entry(name).or_insert(0);
            *total = total.checked_add(n).ok_or("unsupported-construct count overflow")?;
            *entries += 1;
            continue;
        }
        if line == UNSUPPORTED_HEADER {
            unsupported_entries = Some(0);
        } else if let Some(warning) = line.strip_prefix("warning: ") {
            if warning.is_empty() {
                return Err("empty warning diagnostic".to_string());
            }
            output.warnings.push(warning.to_string());
        } else if line.starts_with("warning") {
            return Err(format!("unparsed warning diagnostic: {line}"));
        } else if line.starts_with("error:")
            || line.starts_with("error[")
            || line.starts_with("Error:")
            || line.starts_with("fatal:")
            || line.contains("panicked at")
        {
            return Err(format!("error diagnostic in verification output: {line}"));
        } else if line == "Manual Harness Summary:" {
            if summary_started || output.summary.is_some() {
                return Err("duplicate or misplaced manual summary header".to_string());
            }
            finish_harness(&mut pending, &mut output.harnesses)?;
            summary_started = true;
        } else if let Some(name) = line.strip_prefix("Verification failed for - ") {
            if output.summary.is_some() {
                return Err("failed-harness record follows completion summary".to_string());
            }
            finish_harness(&mut pending, &mut output.harnesses)?;
            summary_started = true;
            if !output.harnesses.get(name).is_some_and(HarnessSummary::is_fail)
                || !reported_failures.insert(name.to_string())
            {
                return Err(format!("contradictory or duplicate failed-harness record: {line}"));
            }
        } else if let Some(description) = line.strip_prefix("Failed Checks: ") {
            let harness = pending.as_mut().ok_or("failed check detail outside a harness")?;
            let failed =
                harness.checks.map(|counts| counts.0).ok_or("failed detail precedes counts")?;
            if description.is_empty()
                || harness.verdict.is_some()
                || harness.failure_details >= failed
            {
                return Err(format!("contradictory or misplaced failed check detail: {line}"));
            }
            // The comparison above proves this increment cannot overflow.
            harness.failure_details += 1;
        } else if line.starts_with("[Kani] info: Verification output shows")
            || line.starts_with("[Kani] tip: Consider increasing the unwinding value")
        {
            if !pending.as_ref().is_some_and(|h| h.verdict == Some(HarnessVerdict::Fail)) {
                return Err("unwinding failure diagnostic without a failed harness".to_string());
            }
        } else if let Some(record) = CHECK_RE.captures(line) {
            if summary_started || output.summary.is_some() {
                return Err("harness record follows completion summary".to_string());
            }
            finish_harness(&mut pending, &mut output.harnesses)?;
            pending = Some(PendingHarness {
                name: record[1].to_string(),
                checks: None,
                covers: None,
                failure_details: 0,
                undetermined_covers: 0,
                unreachable_covers: 0,
                verdict: None,
            });
        } else if let Some(record) = SUMMARY_RE.captures(line) {
            if output.summary.is_some() {
                return Err("duplicate completion summary records found in log".to_string());
            }
            finish_harness(&mut pending, &mut output.harnesses)?;
            let failed: BTreeSet<_> = output
                .harnesses
                .iter()
                .filter(|(_, harness)| harness.is_fail())
                .map(|(name, _)| name.clone())
                .collect();
            if reported_failures != failed {
                return Err("failed-harness summary differs from terminal results".to_string());
            }
            output.summary = Some(VerificationSummary {
                successful: count(&record[1])?,
                failed: count(&record[2])?,
                total: count(&record[3])?,
            });
            output.validate_completion()?;
        } else if line.starts_with("VERIFICATION:-") {
            let harness = pending.as_mut().ok_or("verification verdict outside a harness")?;
            if harness.verdict.is_some() || harness.checks.is_none() {
                return Err(format!("duplicate or misplaced verification verdict: {line}"));
            }
            harness.verdict = Some(match line {
                "VERIFICATION:- SUCCESSFUL" => HarnessVerdict::Pass,
                "VERIFICATION:- FAILED" => HarnessVerdict::Fail,
                _ => return Err(format!("unknown verification verdict: {line}")),
            });
        } else if line.starts_with("**") {
            let harness = pending.as_mut().ok_or("property result outside a harness")?;
            if harness.verdict.is_some() {
                return Err("property result follows its terminal verdict".to_string());
            }
            if let Some(record) = FAILED_RE.captures(line) {
                if harness.checks.is_some() || harness.covers.is_some() {
                    return Err("duplicate or misplaced failure record".to_string());
                }
                let failed = count(&record[1])?;
                let total = count(&record[2])?;
                let undetermined = record.get(3).map_or(Ok(0), |n| count(n.as_str()))?;
                let unreachable = record
                    .get(4)
                    .or_else(|| record.get(5))
                    .map(|n| count(n.as_str()))
                    .transpose()?;
                validate_counts(failed, undetermined, unreachable.unwrap_or(0), total)?;
                harness.checks = Some((failed, total, undetermined, unreachable));
            } else if let Some(record) = COVER_RE.captures(line) {
                if harness.checks.is_none()
                    || harness.covers.is_some()
                    || harness.failure_details != 0
                {
                    return Err("duplicate or misplaced cover record".to_string());
                }
                let satisfied = count(&record[1])?;
                let total = count(&record[2])?;
                let undetermined = record.get(3).map_or(Ok(0), |n| count(n.as_str()))?;
                let unreachable =
                    record.get(4).or_else(|| record.get(5)).map_or(Ok(0), |n| count(n.as_str()))?;
                validate_counts(satisfied, undetermined, unreachable, total)?;
                harness.covers = Some(CoverMetrics { satisfied, total });
                harness.undetermined_covers = undetermined;
                harness.unreachable_covers = unreachable;
            } else {
                return Err(format!("unparsed property result: {line}"));
            }
        } else if line.starts_with("Checking harness ")
            || line.starts_with("Check ")
            || line.starts_with("- Status:")
            || line.starts_with("Failed Checks:")
            || line.starts_with("Verification failed for")
            || line == "RESULTS:"
            || line == "SUMMARY:"
            || line.starts_with("Source-based code coverage results:")
            || line.starts_with("Complete -")
            || line.contains("VERIFICATION:-")
            || (line.starts_with("Thread ") && line.contains("Checking harness"))
            || line.contains("UNDETERMINED")
        {
            return Err(format!("unparsed or unsupported result record: {line}"));
        }
    }
    if unsupported_entries.is_some() {
        return Err("unterminated unsupported-construct diagnostic".to_string());
    }
    finish_harness(&mut pending, &mut output.harnesses)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn sample_consumer_manifest(
        harnesses: Vec<&str>,
        unreachable: BTreeMap<String, u32>,
        disp: Option<&str>,
        warnings: Vec<(&str, &str)>,
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
            expected_cover_properties: 2,
            expected_unreachable_checks: unreachable,
            unreachable_disposition: disp.map(|s| s.to_string()),
            diagnostics: ConsumerDiagnostics {
                warnings: warnings
                    .into_iter()
                    .map(|(m, d)| WarningLedgerEntry {
                        message: m.to_string(),
                        disposition: d.to_string(),
                    })
                    .collect(),
                unsupported_constructs: vec![],
            },
        }
    }

    #[test]
    fn test_parse_success_and_covers() {
        let sample = r#"
Checking harness test::my_harness...
 ** 0 of 2 failed (0 unreachable)
 ** 2 of 2 cover properties satisfied
VERIFICATION:- SUCCESSFUL

Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        assert_eq!(parsed.harnesses.len(), 1);
        let h = &parsed.harnesses["test::my_harness"];
        assert_eq!(h.verdict, HarnessVerdict::Pass);
        assert_eq!(h.unreachable, Some(0));
        assert_eq!(h.covers, Some(CoverMetrics { satisfied: 2, total: 2 }));
        assert_eq!(
            parsed.summary,
            Some(VerificationSummary { successful: 1, failed: 0, total: 1 })
        );
        assert!(parsed.is_pass());
    }

    #[test]
    fn test_exact_declared_unreachable_distribution_passes() {
        let sample = r#"
Checking harness test::h1...
 ** 0 of 5 failed (3 unreachable)
 ** 2 of 2 cover properties satisfied
VERIFICATION:- SUCCESSFUL

Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        let mut unreachable = BTreeMap::new();
        unreachable.insert("test::h1".to_string(), 3);
        let consumer = sample_consumer_manifest(
            vec!["test::h1"],
            unreachable,
            Some("verified loop bound"),
            vec![],
        );
        assert!(parsed.validate_against_consumer(&consumer).is_ok());
    }

    #[test]
    fn test_extra_or_missing_unreachable_check_fails() {
        let sample = r#"
Checking harness test::h1...
 ** 0 of 5 failed (3 unreachable)
 ** 2 of 2 cover properties satisfied
VERIFICATION:- SUCCESSFUL

Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();

        // 1. Extra unreachable check in observed log (manifest expects empty)
        let consumer_empty =
            sample_consumer_manifest(vec!["test::h1"], BTreeMap::new(), None, vec![]);
        assert!(parsed.validate_against_consumer(&consumer_empty).is_err());

        // 2. Missing unreachable check in observed log (manifest expects 5)
        let mut unreachable_diff = BTreeMap::new();
        unreachable_diff.insert("test::h1".to_string(), 5);
        let consumer_diff =
            sample_consumer_manifest(vec!["test::h1"], unreachable_diff, Some("bound"), vec![]);
        assert!(parsed.validate_against_consumer(&consumer_diff).is_err());
    }

    #[test]
    fn test_moved_unreachable_check_to_another_harness_fails() {
        let sample = r#"
Checking harness test::h1...
 ** 0 of 2 failed (0 unreachable)
 ** 1 of 1 cover properties satisfied
VERIFICATION:- SUCCESSFUL
Checking harness test::h2...
 ** 0 of 5 failed (3 unreachable)
 ** 1 of 1 cover properties satisfied
VERIFICATION:- SUCCESSFUL

Complete - 2 successfully verified harnesses, 0 failures, 2 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        let mut expected_unreachable = BTreeMap::new();
        expected_unreachable.insert("test::h1".to_string(), 3); // Manifest expects h1 to have 3 unreachable, but h2 has 3
        let consumer = sample_consumer_manifest(
            vec!["test::h1", "test::h2"],
            expected_unreachable,
            Some("bound"),
            vec![],
        );
        let err = parsed.validate_against_consumer(&consumer).unwrap_err();
        assert!(err.contains("unreachable-check distribution changed"));
    }

    #[test]
    fn test_unledgered_warning_fails_consumer_validation() {
        let sample = r#"
Checking harness test::h1...
 ** 0 of 2 failed (0 unreachable)
 ** 2 of 2 cover properties satisfied
VERIFICATION:- SUCCESSFUL
warning: unexpected unknown warning
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        let consumer = sample_consumer_manifest(vec!["test::h1"], BTreeMap::new(), None, vec![]);
        let err = parsed.validate_against_consumer(&consumer).unwrap_err();
        assert!(err.contains("warning ledger mismatch"));
    }

    #[test]
    fn test_declared_warning_in_ledger_passes() {
        let sample = r#"
Checking harness test::h1...
 ** 0 of 2 failed (0 unreachable)
 ** 2 of 2 cover properties satisfied
VERIFICATION:- SUCCESSFUL
warning: declared warning message
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        let consumer = sample_consumer_manifest(
            vec!["test::h1"],
            BTreeMap::new(),
            None,
            vec![("declared warning message", "accepted per profile RFC")],
        );
        assert!(parsed.validate_against_consumer(&consumer).is_ok());
    }

    #[test]
    fn test_parse_vacuity_cover_with_unreachable() {
        let sample = r#"
Checking harness vacuity...

VERIFICATION RESULT:
 ** 0 of 1 failed (1 unreachable)

 ** 0 of 1 cover properties satisfied (1 unreachable)


VERIFICATION:- SUCCESSFUL
Verification Time: 0.02013746s

Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        assert_eq!(parsed.harnesses.len(), 1);
        let h = &parsed.harnesses["vacuity"];
        assert_eq!(h.verdict, HarnessVerdict::Pass);
        assert_eq!(h.covers, Some(CoverMetrics { satisfied: 0, total: 1 }));
        assert!(!parsed.is_pass());
    }

    #[test]
    fn test_parse_contradictory_harness_output_fails() {
        let sample = r#"
Checking harness test::contradictory...
 ** 1 of 2 failed (0 unreachable)
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("contradictory failure count and verdict"));
    }

    #[test]
    fn test_parse_duplicate_harness_in_single_log_fails() {
        let sample = r#"
Checking harness test::dup...
 ** 0 of 1 failed
VERIFICATION:- SUCCESSFUL
Checking harness test::dup...
 ** 1 of 1 failed
Failed Checks: assertion failed
VERIFICATION:- FAILED
Complete - 1 successfully verified harnesses, 1 failures, 2 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("duplicate harness evidence found in log"));
    }

    #[test]
    fn test_parse_duplicate_summary_fails() {
        let sample = r#"
Checking harness test::h...
 ** 0 of 1 failed
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("duplicate completion summary records found in log"));
    }

    #[test]
    fn test_parse_inconclusive_verdict_fails_closed() {
        let sample = r#"
Checking harness test::aborted_harness...
 ** 0 of 2 failed
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("missing verification verdict"));
    }

    #[test]
    fn test_parse_zero_failure_records_fails() {
        let sample = r#"
Checking harness test::no_records...
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("duplicate or misplaced"));
    }

    #[test]
    fn test_parse_multiple_failure_records_fails() {
        let sample = r#"
Checking harness test::multi_records...
 ** 0 of 1 failed
 ** 0 of 1 failed
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("duplicate or misplaced"));
    }

    #[test]
    fn test_parse_failed_exceeds_total_fails() {
        let sample = r#"
Checking harness test::invalid_counts...
 ** 3 of 2 failed
VERIFICATION:- FAILED
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("result counts exceed"));
    }

    #[test]
    fn test_parse_failed_verdict_with_zero_failed_fails() {
        let sample = r#"
Checking harness test::contradiction_fail...
 ** 0 of 2 failed
VERIFICATION:- FAILED
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("contradictory failure count and verdict"));
    }
}
