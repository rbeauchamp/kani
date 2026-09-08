// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

pub fn validate_sha256(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("expected a lowercase SHA-256 digest".to_string());
    }
    Ok(())
}

fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::CurDir | Component::Normal(_)))
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
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
    pub stderr_sha256: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEntry {
    pub name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlatformManifest {
    pub runner: String,
    pub artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolchainManifest {
    pub schema: u32,
    pub profile: String,
    pub kani_base: String,
    pub kani_tree: String,
    pub cargo_kani_version: String,
    pub cbmc_version: String,
    pub kissat_version: String,
    pub platforms: BTreeMap<String, PlatformManifest>,
}

impl ToolchainManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err(format!("invalid or unsupported toolchain schema: {}", self.schema));
        }
        if self.profile.is_empty() {
            return Err("toolchain manifest profile must be a nonempty string".to_string());
        }
        if self.kani_base.len() != 40 || !self.kani_base.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("toolchain kani_base must be a 40-character hex SHA".to_string());
        }
        if self.kani_tree.len() != 40 || !self.kani_tree.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("toolchain kani_tree must be a 40-character hex SHA".to_string());
        }
        if self.cargo_kani_version.trim().is_empty() {
            return Err("missing cargo_kani_version in toolchain manifest".to_string());
        }
        if self.cbmc_version.trim().is_empty() {
            return Err("missing cbmc_version in toolchain manifest".to_string());
        }
        if self.kissat_version.trim().is_empty() {
            return Err("missing kissat_version in toolchain manifest".to_string());
        }
        if self.platforms.is_empty() {
            return Err("toolchain manifest must define at least one platform".to_string());
        }
        for (pname, platform) in &self.platforms {
            if pname.trim().is_empty() {
                return Err("platform name must be non-empty".to_string());
            }
            if platform.runner.trim().is_empty() {
                return Err(format!("platform {pname} has empty runner"));
            }
            if platform.artifacts.is_empty() {
                return Err(format!("platform {pname} has no artifacts"));
            }
            let mut names = BTreeSet::new();
            for artifact in &platform.artifacts {
                if !relative_path(&artifact.name) || !names.insert(&artifact.name) {
                    return Err(format!("invalid or duplicate artifact name: {}", artifact.name));
                }
                validate_sha256(&artifact.sha256)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WarningLedgerEntry {
    pub message: String,
    pub disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedConstructLedgerEntry {
    pub construct: String,
    pub disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsumerDiagnostics {
    pub warnings: Vec<WarningLedgerEntry>,
    pub unsupported_constructs: Vec<UnsupportedConstructLedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsumerManifest {
    pub schema: u32,
    pub profile: String,
    pub status: String,
    pub consumer: String,
    pub repository: String,
    pub source_commit: String,
    pub source_tree: String,
    pub project_dir: String,
    pub cargo_manifest: String,
    pub cargo_config: Option<String>,
    pub kani_flags: Vec<String>,
    pub expected_harnesses: Vec<String>,
    pub expected_cover_properties: u32,
    pub expected_unreachable_checks: BTreeMap<String, u32>,
    pub unreachable_disposition: Option<String>,
    pub diagnostics: ConsumerDiagnostics,
}

impl ConsumerManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err(format!("invalid or unsupported consumer schema: {}", self.schema));
        }
        if self.profile.is_empty() {
            return Err("consumer manifest profile must be a nonempty string".to_string());
        }
        if self.status != "bootstrap" && self.status != "qualified" {
            return Err(format!(
                "consumer status must be bootstrap or qualified, got: {}",
                self.status
            ));
        }
        if self.source_commit.len() != 40
            || !self.source_commit.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("consumer source_commit must be a full 40-character hex SHA".to_string());
        }
        if self.source_tree.len() != 40 || !self.source_tree.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("consumer source_tree must be a full 40-character hex SHA".to_string());
        }
        if self.expected_harnesses.is_empty() {
            return Err("consumer expected_harnesses must be non-empty".to_string());
        }
        let unique_harnesses: BTreeSet<&String> = self.expected_harnesses.iter().collect();
        if unique_harnesses.len() != self.expected_harnesses.len() {
            return Err("consumer expected_harnesses contains duplicates".to_string());
        }
        if self.expected_harnesses.iter().any(|name| {
            name.is_empty()
                || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b':')
        }) {
            return Err("consumer contains an invalid harness name".to_string());
        }
        if !relative_path(&self.project_dir)
            || !relative_path(&self.cargo_manifest)
            || self.cargo_config.as_deref().is_some_and(|p| !relative_path(p))
        {
            return Err("consumer paths must stay within the declared checkout".to_string());
        }
        if !self.kani_flags.is_empty() {
            return Err(
                "qualified profiles accept the default verification flags only (empty kani_flags)"
                    .to_string(),
            );
        }
        for (harness, count) in &self.expected_unreachable_checks {
            if !unique_harnesses.contains(harness) {
                return Err(format!(
                    "expected_unreachable_checks references unknown harness: {harness}"
                ));
            }
            if *count == 0 {
                return Err(format!(
                    "expected_unreachable_checks for {harness} must be a positive count"
                ));
            }
        }
        if self.expected_unreachable_checks.is_empty() && self.unreachable_disposition.is_some() {
            return Err(
                "zero unreachable checks requires a null unreachable_disposition".to_string()
            );
        }
        if !self.expected_unreachable_checks.is_empty() {
            match &self.unreachable_disposition {
                Some(disp) if !disp.trim().is_empty() => {}
                _ => {
                    return Err(
                        "non-empty unreachable checks require a non-empty unreachable_disposition"
                            .to_string(),
                    );
                }
            }
        }
        // Validate diagnostics ledger uniqueness
        let mut warn_set = BTreeSet::new();
        for w in &self.diagnostics.warnings {
            if w.message.trim().is_empty() || w.disposition.trim().is_empty() {
                return Err("warning ledger entries must have non-empty message and disposition"
                    .to_string());
            }
            if !warn_set.insert(&w.message) {
                return Err(format!("duplicate warning in diagnostics ledger: {}", w.message));
            }
        }
        let mut construct_set = BTreeSet::new();
        for c in &self.diagnostics.unsupported_constructs {
            if c.construct.trim().is_empty() || c.disposition.trim().is_empty() {
                return Err(
                    "unsupported construct ledger entries must have non-empty construct and disposition"
                        .to_string(),
                );
            }
            if !construct_set.insert(&c.construct) {
                return Err(format!(
                    "duplicate unsupported construct in diagnostics ledger: {}",
                    c.construct
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeToolIdentities {
    pub cargo_kani_version: String,
    pub cargo_kani_sha256: String,
    pub kani_sha256: String,
    pub kani_compiler_sha256: String,
    pub cbmc_version: String,
    pub cbmc_sha256: String,
    pub kissat_version: String,
    pub kissat_sha256: String,
}

impl RuntimeToolIdentities {
    pub fn validate(&self) -> Result<(), String> {
        for digest in [
            &self.cargo_kani_sha256,
            &self.kani_sha256,
            &self.kani_compiler_sha256,
            &self.cbmc_sha256,
            &self.kissat_sha256,
        ] {
            validate_sha256(digest)?;
        }
        if [&self.cargo_kani_version, &self.cbmc_version, &self.kissat_version]
            .iter()
            .any(|v| v.trim().is_empty())
        {
            return Err("runtime version observations must be non-empty".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationPurpose {
    InfrastructureSelfTest,
    QualificationMutations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationReceipt {
    pub schema: u32,
    pub purpose: MutationPurpose,
    pub profile: Option<String>,
    pub executed_at: String,
    pub gate_sha256: String,
    pub toolchain_sha256: Option<String>,
    pub observed_runtime: RuntimeToolIdentities,
    pub snapshot_hashes: BTreeMap<String, String>,
    pub warning_dispositions: BTreeMap<String, String>,
    pub fixtures: BTreeMap<String, String>,
    pub results: BTreeMap<String, ProbeResult>,
    pub status: GateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunContext {
    pub consumer_sha256: String,
    pub platform: String,
    pub runtime: RuntimeToolIdentities,
    pub source_commit: String,
    pub source_tree: String,
    pub toolchain_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InputRunEvidence {
    pub context: RunContext,
    pub cwd: String,
    pub expected_harnesses: Vec<String>,
    pub run_id: String,
    pub log_path: String,
    pub sha256: String,
    pub argv: Vec<String>,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminating_signal: Option<i32>,
}

impl InputRunEvidence {
    /// Only the declared default-check invocation can contribute to a receipt.
    /// The producer attests that argv includes all effective configuration flags.
    pub fn validate(
        &self,
        toolchain: &ToolchainManifest,
        consumer: &ConsumerManifest,
        toolchain_sha256: &str,
        consumer_sha256: &str,
    ) -> Result<(), String> {
        if self.run_id.trim().is_empty() || self.exit_code != 0 || self.terminating_signal.is_some()
        {
            return Err(
                "run requires an identity and observed normal successful termination".to_string()
            );
        }
        validate_sha256(&self.sha256)?;
        if self.context.toolchain_sha256 != toolchain_sha256
            || self.context.consumer_sha256 != consumer_sha256
            || self.context.source_commit != consumer.source_commit
            || self.context.source_tree != consumer.source_tree
        {
            return Err(format!(
                "run {} qualification context differs from its manifests",
                self.run_id
            ));
        }
        if !toolchain.platforms.contains_key(&self.context.platform) {
            return Err(format!("run {} uses an undeclared platform", self.run_id));
        }
        crate::mutations::validate_toolchain_identities(&self.context.runtime, toolchain)?;
        let selected: BTreeSet<_> = self.expected_harnesses.iter().collect();
        let allowed: BTreeSet<_> = consumer.expected_harnesses.iter().collect();
        if selected.is_empty()
            || selected.len() != self.expected_harnesses.len()
            || !selected.is_subset(&allowed)
        {
            return Err("run harness selection must be a non-empty unique subset of the profile"
                .to_string());
        }
        let cwd = Path::new(&self.cwd);
        if !cwd.is_absolute() || cwd.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err("run cwd must be an absolute path without parent traversal".to_string());
        }
        let mut argv = vec!["cargo".to_string()];
        if let Some(config) = &consumer.cargo_config {
            argv.extend(["--config".to_string(), cwd.join(config).to_string_lossy().into_owned()]);
        }
        argv.extend([
            "kani".to_string(),
            "--manifest-path".to_string(),
            cwd.join(&consumer.cargo_manifest).to_string_lossy().into_owned(),
            "--exact".to_string(),
            "--output-format=terse".to_string(),
        ]);
        for harness in &self.expected_harnesses {
            argv.extend(["--harness".to_string(), harness.clone()]);
        }
        if self.argv != argv {
            return Err(format!(
                "run {} invocation differs from the admitted qualification command",
                self.run_id
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarnessSummary {
    pub harness: String,
    pub verdict: HarnessVerdict,
    pub failed_checks: u32,
    pub total_checks: u32,
    pub undetermined_checks: u32,
    pub undetermined_covers: u32,
    pub unreachable: Option<u32>,
    pub unreachable_covers: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub covers: Option<CoverMetrics>,
}

impl HarnessSummary {
    pub fn covers_satisfied(&self) -> bool {
        self.undetermined_covers == 0
            && self.unreachable_covers == 0
            && self.covers.is_none_or(|c| c.satisfied == c.total)
    }

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
    pub profile: String,
    pub consumer_status: String,
    pub executed_at: String,
    pub gate_sha256: String,
    pub toolchain_sha256: String,
    pub consumer_sha256: String,
    pub source_commit: String,
    pub source_tree: String,
    pub status: GateStatus,
    pub total_harnesses: u32,
    pub successful_harnesses: u32,
    pub failed_harnesses: u32,
    pub harnesses: BTreeMap<String, HarnessSummary>,
    pub input_runs: Vec<InputRunEvidence>,
    pub cover_properties: CoverMetrics,
    pub unreachable_checks: BTreeMap<String, u32>,
    pub warnings: Vec<String>,
    pub unsupported_constructs: BTreeMap<String, u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_sha256: Option<String>,
}

impl CompositeReceipt {
    pub fn compute_canonical_sha256(&self) -> Result<String, String> {
        let mut cloned = self.clone();
        cloned.receipt_sha256 = None;
        let json = serde_json::to_string(&cloned).map_err(|e| e.to_string())?;
        Ok(crate::mutations::hash_bytes(json.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_toolchain_manifest_deserialization() {
        let raw = include_str!("../../../qualification/manifests/core-v1/toolchain.json");
        let manifest: ToolchainManifest = serde_json::from_str(raw).unwrap();
        manifest.validate().unwrap();
        assert_eq!(manifest.schema, 1);
        assert_eq!(manifest.profile, "core-v1");
        assert!(manifest.cbmc_version.contains("6.10.0"));
        assert!(manifest.platforms.contains_key("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn test_successor_profile_names_accepted() {
        let raw = include_str!("../../../qualification/manifests/core-v1/toolchain.json");
        let mut toolchain: ToolchainManifest = serde_json::from_str(raw).unwrap();
        toolchain.profile = "core-v2".to_string();
        toolchain.validate().unwrap();

        let raw = include_str!("../../../qualification/manifests/core-v2/public-corpus.json");
        let consumer: ConsumerManifest = serde_json::from_str(raw).unwrap();
        assert_eq!(consumer.profile, "core-v2");
        consumer.validate().unwrap();
    }

    #[test]
    fn test_empty_profile_rejected() {
        let raw = include_str!("../../../qualification/manifests/core-v1/toolchain.json");
        let mut toolchain: ToolchainManifest = serde_json::from_str(raw).unwrap();
        toolchain.profile = String::new();
        let err = toolchain.validate().unwrap_err();
        assert!(err.contains("profile must be a nonempty string"));

        let raw = include_str!("../../../qualification/manifests/core-v2/public-corpus.json");
        let mut consumer: ConsumerManifest = serde_json::from_str(raw).unwrap();
        consumer.profile = String::new();
        let err = consumer.validate().unwrap_err();
        assert!(err.contains("profile must be a nonempty string"));
    }

    /// Every checked-in manifest must validate, and each manifest's profile must
    /// equal its profile directory name (mirrors the Python gate test).
    #[test]
    fn test_checked_in_manifests_validate() {
        let manifests = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../qualification/manifests");
        let mut profile_dirs: Vec<_> = fs::read_dir(&manifests)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect();
        profile_dirs.sort();
        assert!(!profile_dirs.is_empty());
        for profile_dir in profile_dirs {
            let profile = profile_dir.file_name().unwrap().to_string_lossy().into_owned();
            let mut manifests: Vec<_> = fs::read_dir(&profile_dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect();
            manifests.sort();
            assert!(!manifests.is_empty(), "{profile} has no manifests");
            for path in manifests {
                let raw = fs::read_to_string(&path).unwrap();
                if path.file_name().unwrap() == "toolchain.json" {
                    let toolchain: ToolchainManifest = serde_json::from_str(&raw).unwrap();
                    assert_eq!(toolchain.profile, profile);
                    toolchain.validate().unwrap();
                } else {
                    let consumer: ConsumerManifest = serde_json::from_str(&raw).unwrap();
                    assert_eq!(consumer.profile, profile);
                    consumer.validate().unwrap();
                }
            }
        }
    }

    #[test]
    fn test_consumer_manifest_validation() {
        let sample = r#"{
            "schema": 1,
            "profile": "core-v1",
            "status": "bootstrap",
            "consumer": "test-pkg",
            "repository": "https://example.invalid/test",
            "source_commit": "0000000000000000000000000000000000000000",
            "source_tree": "1111111111111111111111111111111111111111",
            "project_dir": ".",
            "cargo_manifest": "Cargo.toml",
            "cargo_config": null,
            "kani_flags": [],
            "expected_harnesses": ["h1", "h2"],
            "expected_cover_properties": 2,
            "expected_unreachable_checks": { "h1": 3 },
            "unreachable_disposition": "verified compiler bounds",
            "diagnostics": {
                "warnings": [
                    { "message": "unstable feature", "disposition": "reviewed" }
                ],
                "unsupported_constructs": []
            }
        }"#;
        let consumer: ConsumerManifest = serde_json::from_str(sample).unwrap();
        consumer.validate().unwrap();

        // Unknown field must be rejected
        let invalid_unknown = r#"{
            "schema": 1,
            "profile": "core-v1",
            "status": "bootstrap",
            "consumer": "test-pkg",
            "repository": "https://example.invalid/test",
            "source_commit": "0000000000000000000000000000000000000000",
            "source_tree": "1111111111111111111111111111111111111111",
            "project_dir": ".",
            "cargo_manifest": "Cargo.toml",
            "cargo_config": null,
            "kani_flags": [],
            "expected_harnesses": ["h1"],
            "expected_cover_properties": 0,
            "expected_unreachable_checks": {},
            "unreachable_disposition": null,
            "diagnostics": { "warnings": [], "unsupported_constructs": [] },
            "unknown_extra_field": true
        }"#;
        assert!(serde_json::from_str::<ConsumerManifest>(invalid_unknown).is_err());
    }

    #[test]
    fn test_consumer_manifest_unreachable_disposition_rules() {
        let sample_missing_disp = r#"{
            "schema": 1,
            "profile": "core-v1",
            "status": "bootstrap",
            "consumer": "test-pkg",
            "repository": "https://example.invalid/test",
            "source_commit": "0000000000000000000000000000000000000000",
            "source_tree": "1111111111111111111111111111111111111111",
            "project_dir": ".",
            "cargo_manifest": "Cargo.toml",
            "cargo_config": null,
            "kani_flags": [],
            "expected_harnesses": ["h1"],
            "expected_cover_properties": 0,
            "expected_unreachable_checks": { "h1": 2 },
            "unreachable_disposition": null,
            "diagnostics": { "warnings": [], "unsupported_constructs": [] }
        }"#;
        let consumer: ConsumerManifest = serde_json::from_str(sample_missing_disp).unwrap();
        assert!(consumer.validate().is_err());
    }

    #[test]
    fn test_harness_summary_verdict_roundtrip() {
        let h = HarnessSummary {
            harness: "test::h".to_string(),
            verdict: HarnessVerdict::Pass,
            failed_checks: 0,
            total_checks: 2,
            undetermined_checks: 0,
            undetermined_covers: 0,
            unreachable: Some(0),
            unreachable_covers: 0,
            covers: Some(CoverMetrics { satisfied: 2, total: 2 }),
        };
        let serialized = serde_json::to_string(&h).unwrap();
        let deserialized: HarnessSummary = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.verdict, HarnessVerdict::Pass);
        assert!(deserialized.is_pass());
        assert!(!deserialized.is_fail());
    }
}
