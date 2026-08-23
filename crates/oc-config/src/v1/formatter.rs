//! Ported from: packages/core/src/v1/config/formatter.ts

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OrderedMap;

/// Ported from: packages/core/src/v1/config/formatter.ts:5-10 (Entry)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormatterEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<OrderedMap<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}

/// Ported from: packages/core/src/v1/config/formatter.ts:12-13
/// Info = Union(Boolean, Record(String, Entry))
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum FormatterInfo {
    Bool(bool),
    Entries(OrderedMap<FormatterEntry>),
}

impl<'de> Deserialize<'de> for FormatterInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        serde_json::from_value(value).map_err(serde::de::Error::custom)
    }
}
