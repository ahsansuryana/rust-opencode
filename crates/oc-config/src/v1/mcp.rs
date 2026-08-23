//! Ported from: packages/core/src/v1/config/mcp.ts

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OrderedMap;

use super::server::positive_u64_opt;

/// Union ketiga di config.ts:113-115: Record(String, McpInfo | {enabled: boolean})
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    Local(McpLocal),
    Remote(McpRemote),
    EnabledOnly(McpEnabledOnly),
}

impl<'de> Deserialize<'de> for McpServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        // Urutan union TS: Local → Remote → {enabled}
        if let Ok(local) = serde_json::from_value::<McpLocal>(value.clone()) {
            return Ok(McpServerConfig::Local(local));
        }
        if let Ok(remote) = serde_json::from_value::<McpRemote>(value.clone()) {
            return Ok(McpServerConfig::Remote(remote));
        }
        serde_json::from_value::<McpEnabledOnly>(value)
            .map(McpServerConfig::EnabledOnly)
            .map_err(serde::de::Error::custom)
    }
}

/// Ported from: packages/core/src/v1/config/mcp.ts:26-42 (OAuth)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpOAuth {
    #[serde(rename = "clientId", default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(
        rename = "clientSecret",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(
        rename = "callbackPort",
        default,
        deserialize_with = "callback_port_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub callback_port: Option<u16>,
    #[serde(
        rename = "redirectUri",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub redirect_uri: Option<String>,
}

fn callback_port_opt<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<u16>::deserialize(deserializer)?;
    match value {
        Some(0) => Err(serde::de::Error::custom(
            "Expected integer between 1 and 65535",
        )),
        other => Ok(other),
    }
}

fn literal_local<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value == "local" {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("Expected 'local'"))
    }
}

fn literal_remote<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value == "remote" {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("Expected 'remote'"))
    }
}

/// Ported from: packages/core/src/v1/config/mcp.ts:44-60 (Remote)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRemote {
    #[serde(deserialize_with = "literal_remote")]
    pub r#type: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<OrderedMap<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<McpOAuthOrDisabled>,
    #[serde(
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout: Option<u64>,
}

/// Ported from: packages/core/src/v1/config/mcp.ts:53-55 (oauth = OAuth | false)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum McpOAuthOrDisabled {
    OAuth(McpOAuth),
    Disabled(bool),
}

impl<'de> Deserialize<'de> for McpOAuthOrDisabled {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value == Value::Bool(false) {
            return Ok(McpOAuthOrDisabled::Disabled(false));
        }
        serde_json::from_value::<McpOAuth>(value)
            .map(McpOAuthOrDisabled::OAuth)
            .map_err(serde::de::Error::custom)
    }
}

/// Ported from: packages/core/src/v1/config/mcp.ts:6-24 (Local)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpLocal {
    #[serde(deserialize_with = "literal_local")]
    pub r#type: String,
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<OrderedMap<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout: Option<u64>,
}

/// Ported from: packages/core/src/v1/config/mcp.ts:113-115 sisi `{enabled: boolean}`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpEnabledOnly {
    pub enabled: bool,
}
