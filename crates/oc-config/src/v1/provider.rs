//! Ported from: packages/core/src/v1/config/provider.ts

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OrderedMap;

use super::server::positive_u64_opt;

/// Ported from: packages/core/src/v1/config/provider.ts:6 (ModelStatus)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
    Active,
}

/// Ported from: packages/core/src/v1/config/provider.ts:8-11
/// InterleavedField = Literals([...]) | String ≡ String (validasi tetap string)
pub type InterleavedField = String;

/// Union interleaved: Boolean | InterleavedField | {field}
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Interleaved {
    Bool(bool),
    Field(InterleavedField),
    Object(InterleavedObject),
}

impl<'de> Deserialize<'de> for Interleaved {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Bool(b) => Ok(Interleaved::Bool(b)),
            Value::String(s) => Ok(Interleaved::Field(s)),
            Value::Object(_) => serde_json::from_value::<InterleavedObject>(value)
                .map(Interleaved::Object)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom("Expected interleaved config")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterleavedObject {
    pub field: InterleavedField,
}

/// Ported from: packages/core/src/v1/config/provider.ts:31-46 (cost)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelCostContextOver200k {
    pub input: f64,
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelCost {
    pub input: f64,
    pub output: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    #[serde(
        rename = "context_over_200k",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub context_over_200k: Option<ProviderModelCostContextOver200k>,
}

/// Ported from: packages/core/src/v1/config/provider.ts:47-53 (limit)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelLimit {
    pub context: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    pub output: f64,
}

/// Ported from: packages/core/src/v1/config/provider.ts:54-61 (modalities)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelModalities {
    #[serde(
        default,
        deserialize_with = "modality_list_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub input: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "modality_list_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub output: Option<Vec<String>>,
}

/// Literals(["text","audio","image","video","pdf"])
fn modality_list_opt<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let list = Option::<Vec<String>>::deserialize(deserializer)?;
    if let Some(items) = &list {
        for item in items {
            if !matches!(item.as_str(), "text" | "audio" | "image" | "video" | "pdf") {
                return Err(serde::de::Error::custom(format!(
                    "Expected modality literal, got {item}"
                )));
            }
        }
    }
    Ok(list)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelProvider {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

/// Ported from: packages/core/src/v1/config/provider.ts:72-79 (variants entry)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModelVariant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(flatten)]
    pub rest: OrderedMap<Value>,
}

/// Ported from: packages/core/src/v1/config/provider.ts:13-80 (Model)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(
        rename = "release_date",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,
    #[serde(rename = "tool_call", default, skip_serializing_if = "Option::is_none")]
    pub tool_call: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interleaved: Option<Interleaved>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<ProviderModelCost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<ProviderModelLimit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<ProviderModelModalities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ModelStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderModelProvider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<OrderedMap<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<OrderedMap<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<OrderedMap<ProviderModelVariant>>,
}

/// Timeout = PositiveInt | false
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TimeoutSetting {
    Disabled(bool),
    Milliseconds(u64),
}

impl<'de> Deserialize<'de> for TimeoutSetting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value == Value::Bool(false) {
            return Ok(TimeoutSetting::Disabled(false));
        }
        let ms: u64 = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        if ms == 0 {
            return Err(serde::de::Error::custom("Expected positive integer"));
        }
        Ok(TimeoutSetting::Milliseconds(ms))
    }
}

/// Ported from: packages/core/src/v1/config/provider.ts:90-124
/// options StructWithRest → field dikenal + flatten rest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderOptions {
    #[serde(rename = "apiKey", default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "baseURL", default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(
        rename = "enterpriseUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub enterprise_url: Option<String>,
    #[serde(
        rename = "setCacheKey",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub set_cache_key: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<TimeoutSetting>,
    #[serde(
        rename = "headerTimeout",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub header_timeout: Option<TimeoutSetting>,
    #[serde(
        rename = "chunkTimeout",
        default,
        deserialize_with = "positive_u64_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub chunk_timeout: Option<u64>,
    #[serde(flatten)]
    pub rest: OrderedMap<Value>,
}

/// Ported from: packages/core/src/v1/config/provider.ts:82-126 (Info / ProviderConfig)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whitelist: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blacklist: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<ProviderOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub models: Option<OrderedMap<ProviderModel>>,
}
