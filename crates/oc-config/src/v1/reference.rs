//! Ported from: packages/core/src/config/reference.ts

use serde::{Deserialize, Serialize};

/// Ported from: packages/core/src/config/reference.ts:5-10 (Git)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceGit {
    pub repository: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// Ported from: packages/core/src/config/reference.ts:12-16 (Local)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceLocal {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// Ported from: packages/core/src/config/reference.ts:18 (Entry — urutan union sama)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ReferenceEntry {
    Str(String),
    Git(ReferenceGit),
    Local(ReferenceLocal),
}

impl<'de> Deserialize<'de> for ReferenceEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        // Urutan union TS: String → Git → Local
        match value {
            serde_json::Value::String(s) => Ok(ReferenceEntry::Str(s)),
            other @ serde_json::Value::Object(_) => {
                if let Ok(git) = serde_json::from_value::<ReferenceGit>(other.clone()) {
                    return Ok(ReferenceEntry::Git(git));
                }
                serde_json::from_value::<ReferenceLocal>(other)
                    .map(ReferenceEntry::Local)
                    .map_err(serde::de::Error::custom)
            }
            _ => Err(serde::de::Error::custom("Expected reference entry")),
        }
    }
}

/// Ported from: packages/core/src/config/reference.ts:21 (Info)
/// Map order-preserving meniru objek JS.
pub type ReferenceInfo = crate::v1::OrderedMap<ReferenceEntry>;
