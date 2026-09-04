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
    Regex::new(r"(?m)^\s*\*\* ([0-9]+) of ([0-9]+) cover properties satisfied$").unwrap()
});

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
    /// and the completion summary confirms 0 failures on non-empty harnesses.
    pub fn is_pass(&self) -> bool {
        let all_harnesses_pass = !self.harnesses.is_empty()
            && self
                .harnesses
                .values()
                .all(|h| h.is_pass() && h.covers.map_or(true, |c| c.satisfied == c.total));

        let summary_pass = self.summary.map_or(false, |s| {
            s.failed == 0 && s.successful == s.total && s.total == self.harnesses.len() as u32
        });

        all_harnesses_pass && summary_pass
    }
}

pub fn parse_kani_output(text: &str) -> Result<ParsedOutput, String> {
    let mut harnesses = BTreeMap::new();
    let matches: Vec<_> = CHECK_RE.captures_iter(text).collect();

    for (index, caps) in matches.iter().enumerate() {
        let harness = caps.get(1).ok_or("invalid harness capture")?.as_str().to_string();
        let m = caps.get(0).unwrap();

        let end = if index + 1 < matches.len() {
            matches[index + 1].get(0).unwrap().start()
        } else {
            text.len()
        };
        let block = &text[m.end()..end];

        let successful = block.contains("VERIFICATION:- SUCCESSFUL");
        let failed = block.contains("VERIFICATION:- FAILED");

        let verdict = match (successful, failed) {
            (true, false) => HarnessVerdict::Pass,
            (false, true) => HarnessVerdict::Fail,
            (true, true) => {
                return Err(format!("ambiguous verification status for harness: {harness}"));
            }
            (false, false) => {
                return Err(format!(
                    "missing or inconclusive verification verdict for harness: {harness}"
                ));
            }
        };

        let unreachable = FAILED_RE
            .captures(block)
            .and_then(|c| c.get(3))
            .and_then(|m| m.as_str().parse::<u32>().ok());

        let mut covers = None;
        if let Some(ccaps) = COVER_RE.captures(block) {
            let sat = ccaps.get(1).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            let tot = ccaps.get(2).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            covers = Some(CoverMetrics { satisfied: sat, total: tot });
        }

        harnesses.insert(harness.clone(), HarnessSummary { harness, verdict, unreachable, covers });
    }

    let warnings = WARNING_RE
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

    let summary = SUMMARY_RE.captures(text).and_then(|c| {
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
    fn test_parse_inconclusive_verdict_fails_closed() {
        let sample = r#"
Checking harness test::aborted_harness...
 ** 0 of 2 failed
"#;
        let err = parse_kani_output(sample).unwrap_err();
        assert!(err.contains("missing or inconclusive verification verdict"));
    }
}
