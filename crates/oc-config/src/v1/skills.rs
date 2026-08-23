//! Ported from: packages/core/src/v1/config/skills.ts

use serde::{Deserialize, Serialize};

/// Ported from: packages/core/src/v1/config/skills.ts:5-13 (Info)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillsInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,
}
