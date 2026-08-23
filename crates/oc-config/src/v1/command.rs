//! Ported from: packages/core/src/v1/config/command.ts

use serde::{Deserialize, Serialize};

/// Ported from: packages/core/src/v1/config/command.ts:5-13 (Info)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandInfo {
    pub template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,
}
