//! Ported from: packages/core/src/v1/config/server.ts

use serde::{Deserialize, Deserializer, Serialize};

/// PositiveInt sebagai Option field (u64 > 0).
pub fn positive_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<u64>::deserialize(deserializer)?;
    match value {
        Some(0) => Err(serde::de::Error::custom("Expected positive integer")),
        other => Ok(other),
    }
}

/// Ported from: packages/core/src/v1/config/server.ts:6-19 (Server)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerInfo {
    #[serde(
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub port: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mdns: Option<bool>,
    #[serde(rename = "mdnsDomain", skip_serializing_if = "Option::is_none")]
    pub mdns_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors: Option<Vec<String>>,
}
