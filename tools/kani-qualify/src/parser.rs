// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use crate::model::{
    ConsumerManifest, CoverMetrics, HarnessSummary, HarnessVerdict, VerificationSummary,
};

static CHECK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^Checking harness ([a-zA-Z0-9_:]+)\.\.\.$").unwrap());

static FAILED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*\*\* ([0-9]+) of ([0-9]+) failed(?: \(([0-9]+) unreachable\))?$").unwrap()
});

static COVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^\s*\*\* ([0-9]+) of ([0-9]+) cover properties satisfied(?: \(([0-9]+) unreachable\))?$",
    )
    .unwrap()
});

static VERDICT_SUCCESS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*VERIFICATION:-\s+SUCCESSFUL\s*$").unwrap());

static VERDICT_FAILED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*VERIFICATION:-\s+FAILED\s*$").unwrap());

static WARNING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^warning: (.+)$").unwrap());

static UNSUPPORTED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s+- ([a-zA-Z0-9_ -]+) \(([0-9]+)\)$").unwrap());

static SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^Complete - ([0-9]+) successfully verified (?:functions|harnesses), ([0-9]+) failures, ([0-9]+) total\.$",
    )
    .unwrap()
});

#[derive(Debug, Clone)]
pub struct ParsedOutput {
    pub harnesses: BTreeMap<String, HarnessSummary>,
    pub warnings: Vec<String>,
    pub unsupported_constructs: BTreeMap<String, u32>,
    pub summary: Option<VerificationSummary>,
}

impl ParsedOutput {
    /// Return map of harnesses to positive unreachable check counts.
    pub fn observed_unreachable_map(&self) -> BTreeMap<String, u32> {
        let mut map = BTreeMap::new();
        for (name, h) in &self.harnesses {
            if let Some(u) = h.unreachable {
                if u > 0 {
                    map.insert(name.clone(), u);
                }
            }
        }
        map
    }

    /// Return true if all parsed harnesses passed, all cover obligations are satisfied,
    /// the completion summary confirms 0 failures on non-empty harnesses, and no unledgered
    /// warnings or unsupported constructs are present.
    pub fn is_pass(&self) -> bool {
        let all_harnesses_pass = !self.harnesses.is_empty()
            && self
                .harnesses
                .values()
                .all(|h| h.is_pass() && h.covers.is_none_or(|c| c.satisfied == c.total));

        let summary_pass = self.summary.is_some_and(|s| {
            s.failed == 0 && s.successful == s.total && s.total == self.harnesses.len() as u32
        });

        let no_unledgered_warnings = self.warnings.is_empty();
        let no_unsupported = self.unsupported_constructs.is_empty();

        all_harnesses_pass && summary_pass && no_unledgered_warnings && no_unsupported
    }

    /// Validate the parsed output against the consumer manifest's expected harnesses,
    /// exact unreachable check distribution, cover property obligations, and diagnostic ledgers.
    pub fn validate_against_consumer(&self, consumer: &ConsumerManifest) -> Result<(), String> {
        // 1. Harness inventory match
        let mut observed_harnesses: Vec<_> = self.harnesses.keys().cloned().collect();
        observed_harnesses.sort();
        let mut expected_harnesses = consumer.expected_harnesses.clone();
        expected_harnesses.sort();
        if observed_harnesses != expected_harnesses {
            return Err(format!(
                "harness set mismatch:\nobserved: {observed_harnesses:?}\nexpected: {expected_harnesses:?}"
            ));
        }

        // 2. All harnesses must have PASS verdict
        for (name, h) in &self.harnesses {
            if !h.is_pass() {
                return Err(format!("harness {name} did not pass: verdict is {:?}", h.verdict));
            }
        }

        // 3. Exact unreachable check distribution
        let observed_unreachable = self.observed_unreachable_map();
        if observed_unreachable != consumer.expected_unreachable_checks {
            return Err(format!(
                "unreachable-check distribution changed:\nobserved: {observed_unreachable:?}\nexpected: {:?}",
                consumer.expected_unreachable_checks
            ));
        }

        // 4. Cover property totals and satisfaction
        let total_sat: u32 =
            self.harnesses.values().map(|h| h.covers.map_or(0, |c| c.satisfied)).sum();
        let total_cov: u32 = self.harnesses.values().map(|h| h.covers.map_or(0, |c| c.total)).sum();
        if total_sat != total_cov || total_cov != consumer.expected_cover_properties {
            return Err(format!(
                "cover obligations changed or failed: observed {total_sat}/{total_cov}, expected {}/{}",
                consumer.expected_cover_properties, consumer.expected_cover_properties
            ));
        }

        // 5. Diagnostic warning ledger match
        let expected_warnings: BTreeSet<&String> =
            consumer.diagnostics.warnings.iter().map(|w| &w.message).collect();
        let observed_warnings: BTreeSet<&String> = self.warnings.iter().collect();
        if observed_warnings != expected_warnings {
            return Err(format!(
                "warning ledger mismatch:\nobserved: {observed_warnings:?}\nexpected: {expected_warnings:?}"
            ));
        }

        // 6. Diagnostic unsupported construct ledger match
        let expected_unsupported: BTreeSet<&String> =
            consumer.diagnostics.unsupported_constructs.iter().map(|c| &c.construct).collect();
        let observed_unsupported: BTreeSet<&String> = self.unsupported_constructs.keys().collect();
        if observed_unsupported != expected_unsupported {
            return Err(format!(
                "unsupported-construct ledger mismatch:\nobserved: {observed_unsupported:?}\nexpected: {expected_unsupported:?}"
            ));
        }

        // 7. Completion summary must confirm total matches and 0 failures
        match self.summary {
            Some(s)
                if s.failed == 0
                    && s.successful == s.total
                    && s.total == self.harnesses.len() as u32 =>
            {
                Ok(())
            }
            _ => Err("completion summary missing, invalid, or indicates failures".to_string()),
        }
    }
}

pub fn parse_kani_output(text: &str) -> Result<ParsedOutput, String> {
    let mut harnesses = BTreeMap::new();
    let matches: Vec<_> = CHECK_RE.captures_iter(text).collect();

    for (index, caps) in matches.iter().enumerate() {
        let harness = caps.get(1).ok_or("invalid harness capture")?.as_str().to_string();
        if harnesses.contains_key(&harness) {
            return Err(format!("duplicate harness evidence found in log for: {harness}"));
        }

        let m = caps.get(0).unwrap();

        let end = if index + 1 < matches.len() {
            matches[index + 1].get(0).unwrap().start()
        } else {
            text.len()
        };
        let block = &text[m.end()..end];

        let success_count = VERDICT_SUCCESS_RE.captures_iter(block).count();
        let failed_count = VERDICT_FAILED_RE.captures_iter(block).count();

        let verdict = match (success_count, failed_count) {
            (1, 0) => HarnessVerdict::Pass,
            (0, 1) => HarnessVerdict::Fail,
            (0, 0) => {
                return Err(format!(
                    "missing or inconclusive verification verdict for harness: {harness}"
                ));
            }
            _ => {
                return Err(format!("ambiguous verification status for harness: {harness}"));
            }
        };

        let fail_records: Vec<_> = FAILED_RE.captures_iter(block).collect();
        if fail_records.len() > 1 {
            return Err(format!("duplicate failure records found for harness: {harness}"));
        }

        let unreachable = if let Some(fcaps) = fail_records.first() {
            let num_failed =
                fcaps.get(1).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            if verdict == HarnessVerdict::Pass && num_failed > 0 {
                return Err(format!(
                    "contradictory result for harness {harness}: reported {num_failed} failed checks but verdict is SUCCESSFUL"
                ));
            }
            if verdict == HarnessVerdict::Fail && num_failed == 0 {
                return Err(format!(
                    "contradictory result for harness {harness}: reported 0 failed checks but verdict is FAILED"
                ));
            }
            fcaps.get(3).and_then(|m| m.as_str().parse::<u32>().ok())
        } else {
            None
        };

        let mut covers = None;
        if let Some(ccaps) = COVER_RE.captures(block) {
            let sat = ccaps.get(1).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            let tot = ccaps.get(2).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            covers = Some(CoverMetrics { satisfied: sat, total: tot });
        } else if block.contains("cover properties satisfied") {
            return Err(format!("unparsed cover properties output in harness: {harness}"));
        }

        harnesses.insert(harness.clone(), HarnessSummary { harness, verdict, unreachable, covers });
    }

    let warnings: Vec<String> = WARNING_RE
        .captures_iter(text)
        .map(|c| c.get(1).unwrap().as_str().to_string())
        .filter(|w| !w.starts_with("Found the following unsupported"))
        .collect();

    let mut unsupported_constructs = BTreeMap::new();
    for cap in UNSUPPORTED_RE.captures_iter(text) {
        let construct = cap.get(1).unwrap().as_str().trim().to_string();
        let count = cap.get(2).unwrap().as_str().parse::<u32>().unwrap_or(0);
        *unsupported_constructs.entry(construct).or_insert(0) += count;
    }

    let summary_matches: Vec<_> = SUMMARY_RE.captures_iter(text).collect();
    if summary_matches.len() > 1 {
        return Err("duplicate completion summary records found in log".to_string());
    }

    let summary = summary_matches.first().and_then(|c| {
        let s = c.get(1)?.as_str().parse::<u32>().ok()?;
        let f = c.get(2)?.as_str().parse::<u32>().ok()?;
        let t = c.get(3)?.as_str().parse::<u32>().ok()?;
        Some(VerificationSummary { successful: s, failed: f, total: t })
    });

    Ok(ParsedOutput { harnesses, warnings, unsupported_constructs, summary })
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
 ** 0 of 2 failed (3 unreachable)
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
 ** 0 of 2 failed (3 unreachable)
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
 ** 0 of 2 failed (3 unreachable)
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
        assert!(err.contains("contradictory result for harness"));
    }

    #[test]
    fn test_parse_duplicate_harness_in_single_log_fails() {
        let sample = r#"
Checking harness test::dup...
VERIFICATION:- SUCCESSFUL
Checking harness test::dup...
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
        assert!(err.contains("missing or inconclusive verification verdict"));
    }
}
