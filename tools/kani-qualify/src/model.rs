// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HarnessVerdict {
    Pass,
    Fail,
}

impl std::fmt::Display for HarnessVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessVerdict::Pass => write!(f, "PASS"),
            HarnessVerdict::Fail => write!(f, "FAIL"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverMetrics {
    pub satisfied: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationSummary {
    pub successful: u32,
    pub failed: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProbeResult {
    pub detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returncode: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kani_returncode: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positive_control_returncode: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complete_output_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated_output_sha256: Option<String>,
}

impl ProbeResult {
    pub fn new(detected: bool, failure_evidence: &str) -> Self {
        Self {
            detected,
            evidence: (!detected).then(|| failure_evidence.to_string()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationReceipt {
    pub schema: u32,
    pub executed_at: String,
    pub gate_sha256: String,
    pub toolchain_sha256: String,
    pub fixtures: BTreeMap<String, String>,
    pub results: BTreeMap<String, ProbeResult>,
    pub status: GateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainManifest {
    pub schema: Option<u32>,
    pub profile: Option<String>,
    #[serde(alias = "cargo_kani_version")]
    pub kani: Option<String>,
    pub rustc: Option<String>,
    #[serde(alias = "cbmc_version")]
    pub cbmc: Option<String>,
    #[serde(alias = "kissat_version")]
    pub kissat: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl ToolchainManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != Some(1) {
            return Err(format!("invalid or unsupported toolchain schema: {:?}", self.schema));
        }
        if let Some(prof) = &self.profile
            && prof != "core-v1"
        {
            return Err(format!("unsupported qualification profile: {prof}"));
        }
        let kani = self.kani.as_deref().unwrap_or("");
        if kani.is_empty() || !kani.contains("0.67") {
            return Err(format!("invalid or unverified kani version: {kani:?}"));
        }
        let cbmc = self.cbmc.as_deref().unwrap_or("");
        if cbmc.is_empty() || cbmc == "0.0.0" {
            return Err("missing or empty cbmc version in toolchain manifest".to_string());
        }
        let kissat = self.kissat.as_deref().unwrap_or("");
        if kissat.is_empty() || kissat == "0.0.0" {
            return Err("missing or empty kissat version in toolchain manifest".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessSummary {
    pub harness: String,
    pub verdict: HarnessVerdict,
    pub unreachable: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub covers: Option<CoverMetrics>,
}

impl HarnessSummary {
    pub fn is_pass(&self) -> bool {
        self.verdict == HarnessVerdict::Pass
    }

    #[allow(dead_code)]
    pub fn is_fail(&self) -> bool {
        self.verdict == HarnessVerdict::Fail
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeReceipt {
    pub schema: u32,
    pub executed_at: String,
    pub status: GateStatus,
    pub total_harnesses: u32,
    pub successful_harnesses: u32,
    pub failed_harnesses: u32,
    pub harnesses: BTreeMap<String, HarnessSummary>,
    pub warnings: Vec<String>,
    pub unsupported_constructs: BTreeMap<String, u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_logs: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_sha256: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolchain_manifest_aliases() {
        let sample = r#"{
            "schema": 1,
            "profile": "core-v1",
            "cargo_kani_version": "0.67.0",
            "cbmc_version": "6.10.0 (cbmc-6.10.0)",
            "kissat_version": "4.0.1"
        }"#;
        let manifest: ToolchainManifest = serde_json::from_str(sample).unwrap();
        assert_eq!(manifest.schema, Some(1));
        assert_eq!(manifest.kani.as_deref(), Some("0.67.0"));
        assert_eq!(manifest.cbmc.as_deref(), Some("6.10.0 (cbmc-6.10.0)"));
        assert_eq!(manifest.kissat.as_deref(), Some("4.0.1"));
    }

    #[test]
    fn test_harness_summary_verdict_roundtrip() {
        let h = HarnessSummary {
            harness: "test::h".to_string(),
            verdict: HarnessVerdict::Pass,
            unreachable: Some(0),
            covers: Some(CoverMetrics { satisfied: 2, total: 2 }),
        };
        let serialized = serde_json::to_string(&h).unwrap();
        let deserialized: HarnessSummary = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.verdict, HarnessVerdict::Pass);
        assert!(deserialized.is_pass());
        assert!(!deserialized.is_fail());
    }
}
