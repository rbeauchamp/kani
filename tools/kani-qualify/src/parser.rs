// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::model::{CoverMetrics, HarnessSummary, HarnessVerdict, VerificationSummary};

static CHECK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:Thread [0-9]+: )?Checking harness ([A-Za-z0-9_:]+)\.\.\.$").unwrap()
});

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

#[derive(Debug)]
pub struct ParsedOutput {
    pub harnesses: BTreeMap<String, HarnessSummary>,
    pub warnings: Vec<String>,
    pub unsupported_constructs: BTreeMap<String, u32>,
    pub summary: Option<VerificationSummary>,
}

impl ParsedOutput {
    /// Return true only if all parsed harnesses passed, all cover obligations are satisfied,
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
        assert!(!parsed.is_pass()); // Satisfied (0) < Total (1) must not pass
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
    fn test_parse_failure_with_unreachable() {
        let sample = r#"
Checking harness test::failing_harness...
 ** 1 of 5 failed (2 unreachable)
VERIFICATION:- FAILED

warning: Found the following unsupported constructs:
 - inline assembly (1)
"#;
        let parsed = parse_kani_output(sample).unwrap();
        let h = &parsed.harnesses["test::failing_harness"];
        assert_eq!(h.verdict, HarnessVerdict::Fail);
        assert_eq!(h.unreachable, Some(2));
        assert_eq!(parsed.unsupported_constructs.get("inline assembly"), Some(&1));
        assert!(!parsed.is_pass());
    }

    #[test]
    fn test_unledgered_warnings_fail_gate() {
        let sample = r#"
Checking harness test::warn_harness...
VERIFICATION:- SUCCESSFUL
warning: unledgered compiler warning
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
"#;
        let parsed = parse_kani_output(sample).unwrap();
        assert!(!parsed.warnings.is_empty());
        assert!(!parsed.is_pass());
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
