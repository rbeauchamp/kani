// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::model::HarnessSummary;

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
    LazyLock::new(|| Regex::new(r"^\s+- ([a-zA-Z0-9_ -]+) \(([0-9]+)\)$").unwrap());

static SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^Complete - ([0-9]+) successfully verified (?:functions|harnesses), ([0-9]+) failures, ([0-9]+) total\.$",
    )
    .unwrap()
});

pub struct ParsedOutput {
    pub harnesses: BTreeMap<String, HarnessSummary>,
    pub warnings: Vec<String>,
    pub unsupported_constructs: BTreeMap<String, u32>,
    pub summary: Option<(u32, u32, u32)>, // (success, failures, total)
}

pub fn parse_kani_output(text: &str) -> Result<ParsedOutput, String> {
    let mut harnesses = BTreeMap::new();
    let matches: Vec<_> = CHECK_RE.find_iter(text).collect();

    for (index, m) in matches.iter().enumerate() {
        let caps = CHECK_RE.captures(m.as_str()).ok_or("invalid harness capture")?;
        let harness = caps.get(1).unwrap().as_str().to_string();

        let end = if index + 1 < matches.len() { matches[index + 1].start() } else { text.len() };
        let block = &text[m.end()..end];

        let successful = block.contains("VERIFICATION:- SUCCESSFUL");
        let failed = block.contains("VERIFICATION:- FAILED");

        if successful == failed && (successful || failed) {
            return Err(format!("ambiguous verification status for harness: {harness}"));
        }

        let mut unreachable = None;
        if let Some(fcaps) = FAILED_RE.captures(block) {
            if let Some(unreach) = fcaps.get(3) {
                unreachable = unreach.as_str().parse::<u32>().ok();
            }
        }

        let mut covers_satisfied = None;
        let mut covers_total = None;
        if let Some(ccaps) = COVER_RE.captures(block) {
            let sat = ccaps.get(1).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            let tot = ccaps.get(2).unwrap().as_str().parse::<u32>().map_err(|e| e.to_string())?;
            covers_satisfied = Some(sat);
            covers_total = Some(tot);
        }

        harnesses.insert(
            harness.clone(),
            HarnessSummary {
                harness,
                successful,
                failed,
                unreachable,
                covers_satisfied,
                covers_total,
            },
        );
    }

    let mut warnings = Vec::new();
    for cap in WARNING_RE.captures_iter(text) {
        let w = cap.get(1).unwrap().as_str().to_string();
        if !w.starts_with("Found the following unsupported") {
            warnings.push(w);
        }
    }

    let mut unsupported_constructs = BTreeMap::new();
    for line in text.lines() {
        if let Some(cap) = UNSUPPORTED_RE.captures(line) {
            let construct = cap.get(1).unwrap().as_str().trim().to_string();
            let count = cap.get(2).unwrap().as_str().parse::<u32>().unwrap_or(0);
            *unsupported_constructs.entry(construct).or_insert(0) += count;
        }
    }

    let summary = SUMMARY_RE.captures(text).and_then(|c| {
        let s = c.get(1)?.as_str().parse::<u32>().ok()?;
        let f = c.get(2)?.as_str().parse::<u32>().ok()?;
        let t = c.get(3)?.as_str().parse::<u32>().ok()?;
        Some((s, f, t))
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
        assert!(h.successful);
        assert!(!h.failed);
        assert_eq!(h.unreachable, Some(0));
        assert_eq!(h.covers_satisfied, Some(2));
        assert_eq!(h.covers_total, Some(2));
        assert_eq!(parsed.summary, Some((1, 0, 1)));
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
        assert!(h.failed);
        assert!(!h.successful);
        assert_eq!(h.unreachable, Some(2));
        assert_eq!(parsed.unsupported_constructs.get("inline assembly"), Some(&1));
    }
}
