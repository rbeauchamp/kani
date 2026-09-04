// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub kani: Option<String>,
    pub rustc: Option<String>,
    pub cbmc: Option<String>,
    pub kissat: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSummary {
    pub harness: String,
    pub successful: bool,
    pub failed: bool,
    pub unreachable: Option<u32>,
    pub covers_satisfied: Option<u32>,
    pub covers_total: Option<u32>,
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
}
