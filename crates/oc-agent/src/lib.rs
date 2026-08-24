//! Ported dari packages/opencode/src/agent/agent.ts

pub mod registry;

/// Ported from: agent.ts:35-56 (Info schema)
#[derive(Debug, Clone)]
pub struct Info {
    pub name: String,
    pub description: Option<String>,
    /// "subagent" | "primary" | "all"
    pub mode: Mode,
    pub native: bool,
    pub hidden: bool,
    pub top_p: Option<f64>,
    pub temperature: Option<f64>,
    pub color: Option<String>,
    pub permission: Vec<oc_permission::Rule>,
    pub model: Option<ModelRef>,
    pub variant: Option<String>,
    pub prompt: Option<String>,
    pub options: std::collections::BTreeMap<String, serde_json::Value>,
    pub steps: Option<u64>,
}

impl Default for Info {
    fn default() -> Self {
        Info {
            name: String::new(),
            description: None,
            mode: Mode::All,
            native: false,
            hidden: false,
            top_p: None,
            temperature: None,
            color: None,
            permission: Vec::new(),
            model: None,
            variant: None,
            prompt: None,
            options: Default::default(),
            steps: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Subagent,
    Primary,
    All,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Subagent => "subagent",
            Mode::Primary => "primary",
            Mode::All => "all",
        }
    }

    pub fn from_config_str(s: &str) -> Self {
        match s {
            "subagent" => Mode::Subagent,
            "primary" => Mode::Primary,
            _ => Mode::All,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
}

/// Config override shape dari cfg.agent (config.ts ConfigAgent).
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct AgentConfigOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(rename = "top_p", default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<serde_json::Value>,
}
