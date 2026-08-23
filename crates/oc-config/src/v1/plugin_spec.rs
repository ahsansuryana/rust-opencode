//! Ported from: packages/core/src/v1/config/plugin.ts

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OrderedMap;

/// Ported from: packages/core/src/v1/config/plugin.ts:5-6 (Options)
pub type PluginOptions = OrderedMap<Value>;

/// Ported from: packages/core/src/v1/config/plugin.ts:8-9
/// Spec = Union(String, [String, Options])
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PluginSpec {
    Str(String),
    Pair(String, PluginOptions),
}

impl<'de> Deserialize<'de> for PluginSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::String(s) => Ok(PluginSpec::Str(s)),
            Value::Array(items) if items.len() == 2 => {
                let name = match &items[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(serde::de::Error::custom(
                            "Expected plugin spec tuple [string, options]",
                        ))
                    }
                };
                let options = match &items[1] {
                    Value::Object(map) => {
                        serde_json::from_value::<OrderedMap<Value>>(Value::Object(map.clone()))
                            .map_err(serde::de::Error::custom)?
                    }
                    _ => {
                        return Err(serde::de::Error::custom(
                            "Expected plugin spec tuple [string, options]",
                        ))
                    }
                };
                Ok(PluginSpec::Pair(name, options))
            }
            _ => Err(serde::de::Error::custom(
                "Expected plugin spec string or [string, options] tuple",
            )),
        }
    }
}
