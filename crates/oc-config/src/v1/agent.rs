//! Ported from: packages/core/src/v1/config/agent.ts

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::permission::PermissionInfo;
use super::OrderedMap;

/// Ported from: packages/core/src/v1/config/agent.ts:26 (mode literals)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Subagent,
    Primary,
    All,
}

/// Ported from: packages/core/src/v1/config/agent.ts:7-10 (Color)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Color {
    Theme(ColorTheme),
    Hex(String),
}

/// Ported from: packages/core/src/v1/config/agent.ts:7-10
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorTheme {
    #[serde(rename = "primary")]
    Primary,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "accent")]
    Accent,
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "info")]
    Info,
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Some(hex) = value.strip_prefix('#') {
            // pattern /^#[0-9a-fA-F]{6}$/
            let valid = value.len() == 7 && hex.bytes().all(|b| b.is_ascii_hexdigit());
            if !valid {
                return Err(serde::de::Error::custom(
                    "Expected color to match ^#[0-9a-fA-F]{6}$",
                ));
            }
            Ok(Color::Hex(value))
        } else {
            match serde_json::from_value::<ColorTheme>(Value::String(value.clone())) {
                Ok(theme) => Ok(Color::Theme(theme)),
                Err(_) => Err(serde::de::Error::custom(
                    "Expected color to be a hex code or theme color",
                )),
            }
        }
    }
}

/// Bentuk mentah `AgentSchema` sebelum transform normalize.
/// StructWithRest → field dikenal + flatten rest.
#[derive(Debug, Clone, Default)]
pub struct AgentConfigRaw {
    pub model: Option<String>,
    pub variant: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub prompt: Option<String>,
    pub tools: Option<OrderedMap<bool>>,
    pub disable: Option<bool>,
    pub description: Option<String>,
    pub mode: Option<AgentMode>,
    pub hidden: Option<bool>,
    pub options: Option<OrderedMap<Value>>,
    pub color: Option<Color>,
    pub steps: Option<u64>,
    pub max_steps: Option<u64>,
    pub permission: Option<PermissionInfo>,
    pub rest: OrderedMap<Value>,
}

const RAW_FIELDS: &[&str] = &[
    "model",
    "variant",
    "temperature",
    "top_p",
    "prompt",
    "tools",
    "disable",
    "description",
    "mode",
    "hidden",
    "options",
    "color",
    "steps",
    "maxSteps",
    "permission",
];

impl<'de> Deserialize<'de> for AgentConfigRaw {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        let Value::Object(map) = value else {
            return Err(Error::custom("Expected agent config object"));
        };
        let take_string = |m: &serde_json::Map<String, serde_json::Value>, key: &str| {
            m.get(key)
                .filter(|v| !v.is_null())
                .and_then(|v| v.as_str().map(str::to_string))
        };
        let take_bool = |m: &serde_json::Map<String, serde_json::Value>, key: &str| {
            m.get(key)
                .filter(|v| !v.is_null())
                .and_then(|v| v.as_bool())
        };
        let take_f64 = |m: &serde_json::Map<String, serde_json::Value>, key: &str| {
            m.get(key).filter(|v| !v.is_null()).and_then(|v| v.as_f64())
        };
        let take_u64 = |m: &serde_json::Map<String, serde_json::Value>,
                        key: &str|
         -> Result<Option<u64>, D::Error> {
            let Some(v) = m.get(key).filter(|v| !v.is_null()) else {
                return Ok(None);
            };
            let Some(n) = v.as_u64() else {
                return Err(Error::custom("Expected positive integer"));
            };
            if n == 0 {
                return Err(Error::custom("Expected positive integer"));
            }
            Ok(Some(n))
        };
        let raw = AgentConfigRaw {
            model: take_string(&map, "model"),
            variant: take_string(&map, "variant"),
            temperature: take_f64(&map, "temperature"),
            top_p: take_f64(&map, "top_p"),
            prompt: take_string(&map, "prompt"),
            tools: map
                .get("tools")
                .filter(|v| !v.is_null())
                .map(|v| {
                    serde_json::from_value::<OrderedMap<bool>>(v.clone()).map_err(Error::custom)
                })
                .transpose()?,
            disable: take_bool(&map, "disable"),
            description: take_string(&map, "description"),
            mode: map
                .get("mode")
                .filter(|v| !v.is_null())
                .map(|v| serde_json::from_value::<AgentMode>(v.clone()).map_err(Error::custom))
                .transpose()?,
            hidden: take_bool(&map, "hidden"),
            options: map
                .get("options")
                .filter(|v| !v.is_null())
                .map(|v| {
                    serde_json::from_value::<OrderedMap<Value>>(v.clone()).map_err(Error::custom)
                })
                .transpose()?,
            color: map
                .get("color")
                .filter(|v| !v.is_null())
                .map(|v| serde_json::from_value::<Color>(v.clone()).map_err(Error::custom))
                .transpose()?,
            steps: take_u64(&map, "steps")?,
            max_steps: take_u64(&map, "maxSteps")?,
            permission: map
                .get("permission")
                .filter(|v| !v.is_null())
                .map(|v| serde_json::from_value::<PermissionInfo>(v.clone()).map_err(Error::custom))
                .transpose()?,
            rest: map.into_iter().collect::<OrderedMap<serde_json::Value>>(),
        };
        Ok(raw)
    }
}

/// Ported from: packages/core/src/v1/config/agent.ts:12-41 (AgentSchema / Info)
/// Deserialize menjalankan transform `normalize` persis seperti decodeTo di TS.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<OrderedMap<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<AgentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OrderedMap<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u64>,
    #[serde(rename = "maxSteps", skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionInfo>,
    #[serde(flatten)]
    pub rest: OrderedMap<Value>,
}

impl<'de> Deserialize<'de> for AgentConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = AgentConfigRaw::deserialize(deserializer)?;
        Ok(agent_normalize(raw))
    }
}

/// Ported from: packages/core/src/v1/config/agent.ts:43-60 (KNOWN_KEYS)
const KNOWN_KEYS: &[&str] = &[
    "name",
    "model",
    "variant",
    "prompt",
    "description",
    "temperature",
    "top_p",
    "mode",
    "hidden",
    "color",
    "steps",
    "maxSteps",
    "options",
    "permission",
    "disable",
    "tools",
];

/// Ported from: packages/core/src/v1/config/agent.ts:62-81 (normalize)
pub fn agent_normalize(mut agent: AgentConfigRaw) -> AgentConfig {
    // TS: options = { ...agent.options }; lalu key non-KNOWN_KEYS di-copy ke
    // options — key tetap ada flat karena spread `...agent` dipertahankan.
    let mut options: OrderedMap<Value> = agent.options.take().unwrap_or_default();
    for (key, value) in &agent.rest.entries {
        if !KNOWN_KEYS.contains(&key.as_str()) && !RAW_FIELDS.contains(&key.as_str()) {
            options.insert(key.clone(), value.clone());
        }
    }

    let mut permission = PermissionInfo::empty();
    if let Some(tools) = &agent.tools {
        for (tool, enabled) in tools.iter() {
            let action = if *enabled { "allow" } else { "deny" };
            if tool == "write" || tool == "edit" || tool == "patch" {
                permission.set_edit(action);
                continue;
            }
            permission.insert_rule(tool.clone(), action);
        }
    }
    if let Some(existing) = agent.permission.take() {
        permission.assign_from(existing);
    }

    let steps = agent.steps.or(agent.max_steps);

    // Rest hanya menyimpan key yang TIDAK dideklarasikan sebagai field
    // (mis. `name`) supaya serialisasi tidak menduplikasi key.
    let mut rest = std::mem::take(&mut agent.rest);
    rest.entries
        .retain(|(key, _)| !RAW_FIELDS.contains(&key.as_str()));

    AgentConfig {
        model: agent.model,
        variant: agent.variant,
        temperature: agent.temperature,
        top_p: agent.top_p,
        prompt: agent.prompt,
        tools: agent.tools,
        disable: agent.disable,
        description: agent.description,
        mode: agent.mode,
        hidden: agent.hidden,
        options: Some(options),
        color: agent.color,
        steps,
        max_steps: agent.max_steps,
        permission: Some(permission),
        rest,
    }
}
