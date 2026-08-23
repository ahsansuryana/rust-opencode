//! Ported from: packages/core/src/config/experimental.ts (+ policy.ts, catalog.ts)

use serde::{Deserialize, Serialize};

/// Ported from: packages/core/src/catalog.ts:20 (PolicyActions — literal tunggal)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyAction {
    #[serde(rename = "provider.use")]
    ProviderUse,
}

/// Ported from: packages/core/src/policy.ts:6-7 (Effect)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Ported from: packages/core/src/policy.ts:10-14 (Policy.Info fields)
/// + action dipersempit jadi PolicyAction di experimental.ts:11-14.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalPolicy {
    pub action: PolicyAction,
    pub effect: PolicyEffect,
    pub resource: String,
}

/// Ported from: packages/core/src/v1/config/config.ts:169-189 (experimental)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperimentalInfo {
    #[serde(
        rename = "disable_paste_summary",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub disable_paste_summary: Option<bool>,
    #[serde(
        rename = "batch_tool",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub batch_tool: Option<bool>,
    #[serde(
        rename = "openTelemetry",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub open_telemetry: Option<bool>,
    #[serde(
        rename = "primary_tools",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub primary_tools: Option<Vec<String>>,
    #[serde(
        rename = "continue_loop_on_deny",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub continue_loop_on_deny: Option<bool>,
    #[serde(
        rename = "mcp_timeout",
        default,
        deserialize_with = "positive_u64_opt_local",
        skip_serializing_if = "Option::is_none"
    )]
    pub mcp_timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policies: Option<Vec<ExperimentalPolicy>>,
}

fn positive_u64_opt_local<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<u64>::deserialize(deserializer)?;
    match value {
        Some(0) => Err(serde::de::Error::custom("Expected positive integer")),
        other => Ok(other),
    }
}
