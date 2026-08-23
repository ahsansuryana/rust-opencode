//! Ported from: packages/core/src/v1/config/config.ts (schema Info "Config")

use crate::v1::OrderedMap;
use serde::{Deserialize, Serialize};

use super::agent::AgentConfig;
use super::attachment::AttachmentInfo;
use super::command::CommandInfo;
use super::experimental::ExperimentalInfo;
use super::formatter::FormatterInfo;
use super::layout::Layout;
use super::lsp::LspInfo;
use super::mcp::McpServerConfig;
use super::permission::PermissionInfo;
use super::plugin_spec::PluginSpec;
use super::provider::ProviderInfo;
use super::reference::ReferenceInfo;
use super::server::{positive_u64_opt, ServerInfo};
use super::skills::SkillsInfo;

/// Ported from: packages/core/src/v1/config/config.ts:27-30
/// (LogLevelRef — literal "DEBUG"|"INFO"|"WARN"|"ERROR")
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "DEBUG")]
    Debug,
    #[serde(rename = "INFO")]
    Info,
    #[serde(rename = "WARN")]
    Warn,
    #[serde(rename = "ERROR")]
    Error,
}

/// Ported from: packages/core/src/v1/config/config.ts:57-60 (share literals)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareMode {
    Manual,
    Auto,
    Disabled,
}

/// Ported from: packages/core/src/v1/config/config.ts:64-67
/// autoupdate = Boolean | "notify"
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AutoUpdate {
    Bool(bool),
    Notify,
}

impl<'de> Deserialize<'de> for AutoUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Bool(b) => Ok(AutoUpdate::Bool(b)),
            serde_json::Value::String(s) if s == "notify" => Ok(AutoUpdate::Notify),
            _ => Err(serde::de::Error::custom("Expected boolean or 'notify'")),
        }
    }
}

/// Ported from: packages/core/src/v1/config/config.ts:51 (watcher)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatcherInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
}

/// Ported from: packages/core/src/v1/config/config.ts:133-135 (enterprise)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnterpriseInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Ported from: packages/core/src/v1/config/config.ts:136-148 (tool_output)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolOutputInfo {
    #[serde(
        rename = "max_lines",
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_lines: Option<u64>,
    #[serde(
        rename = "max_bytes",
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_bytes: Option<u64>,
}

/// Ported from: packages/core/src/v1/config/config.ts:149-168 (compaction)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompactionInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prune: Option<bool>,
    #[serde(
        rename = "tail_turns",
        default,
        deserialize_with = "super::non_negative_int_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub tail_turns: Option<u64>,
    #[serde(
        rename = "preserve_recent_tokens",
        default,
        deserialize_with = "super::non_negative_int_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub preserve_recent_tokens: Option<u64>,
    #[serde(
        default,
        deserialize_with = "super::non_negative_int_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub reserved: Option<u64>,
}

/// Ported from: packages/core/src/v1/config/config.ts:32-190 (Info / Config)
///
/// Semua excess property diabaikan sesuai `onExcessProperty: "ignore"` di
/// config/parse.ts. Urutan deklarasi field mengikuti urutan TS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Info {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(rename = "logLevel", default, skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<OrderedMap<CommandInfo>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<SkillsInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<ReferenceInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<ReferenceInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watcher: Option<WatcherInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<Vec<PluginSpec>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<ShareMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoshare: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoupdate: Option<AutoUpdate>,
    #[serde(
        rename = "disabled_providers",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub disabled_providers: Option<Vec<String>>,
    #[serde(
        rename = "enabled_providers",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled_providers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(
        rename = "small_model",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub small_model: Option<String>,
    #[serde(
        rename = "default_agent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_agent: Option<String>,
    #[serde(
        rename = "subagent_depth",
        default,
        deserialize_with = "super::non_negative_int_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub subagent_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<OrderedMap<AgentConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<OrderedMap<AgentConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<OrderedMap<ProviderInfo>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<OrderedMap<McpServerConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatter: Option<FormatterInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lsp: Option<LspInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<OrderedMap<bool>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<AttachmentInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enterprise: Option<EnterpriseInfo>,
    #[serde(
        rename = "tool_output",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tool_output: Option<ToolOutputInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction: Option<CompactionInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<ExperimentalInfo>,
}
