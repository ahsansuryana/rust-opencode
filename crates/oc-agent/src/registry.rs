//! Ported dari agent.ts:88-353 (layer state) — built-in agents + config
//! override + permission merge + default resolution.

use std::collections::BTreeMap;

use oc_permission::Action;

use crate::{AgentConfigOverride, Info, Mode, ModelRef};

pub const PROMPT_COMPACTION: &str = include_str!("../prompts/compaction.txt");
pub const PROMPT_EXPLORE: &str = include_str!("../prompts/explore.txt");
pub const PROMPT_TITLE: &str = include_str!("../prompts/title.txt");
pub const PROMPT_SUMMARY: &str = include_str!("../prompts/summary.txt");
pub const PROMPT_GENERATE: &str = include_str!("../prompts/generate.txt");

/// Ported dari defaults Permission.fromConfig (agent.ts:119-136).
pub fn default_permission_rules() -> Vec<oc_permission::Rule> {
    let config = serde_json::json!({
        "*": "allow",
        "doom_loop": "ask",
        "question": "deny",
        "plan_enter": "deny",
        "plan_exit": "deny",
        "read": {
            "*": "allow",
            "*.env": "ask",
            "*.env.*": "ask",
            "*.env.example": "allow"
        }
    });
    oc_permission::from_config_value(&config)
}

/// Ported dari agent.ts:140-265 — bangun seluruh built-in agents.
pub fn build_agents(
    data_dir: &std::path::Path,
    tmp_dir: &std::path::Path,
    user_ruleset: Vec<oc_permission::Rule>,
    config_overrides: &BTreeMap<String, AgentConfigOverride>,
) -> BTreeMap<String, Info> {
    let defaults = default_permission_rules();
    let mut agents: BTreeMap<String, Info> = BTreeMap::new();

    let plans_glob = format!("{}/plans/*", data_dir.display()).replace('\\', "/");

    // build
    agents.insert(
        "build".to_string(),
        Info {
            name: "build".into(),
            description: Some(
                "The default agent. Executes tools based on configured permissions.".into(),
            ),
            permission: oc_permission::merge_rulesets(&[
                defaults.clone(),
                oc_permission::from_config_value(
                    &serde_json::json!({"question":"allow","plan_enter":"allow"}),
                ),
                user_ruleset.clone(),
            ]),
            mode: Mode::Primary,
            native: true,
            ..Default::default()
        },
    );

    // plan
    {
        let mut ext_dir = serde_json::Map::new();
        ext_dir.insert(plans_glob.clone(), serde_json::json!("allow"));
        let mut edit_map = serde_json::Map::new();
        edit_map.insert("*".into(), serde_json::json!("deny"));
        let plan_config = serde_json::Value::Object({
            let mut m = serde_json::Map::new();
            m.insert("question".into(), serde_json::json!("allow"));
            m.insert("plan_exit".into(), serde_json::json!("allow"));
            m.insert("task".into(), serde_json::json!({ "general": "deny" }));
            m.insert(
                "external_directory".into(),
                serde_json::Value::Object(ext_dir),
            );
            m.insert("edit".into(), serde_json::Value::Object(edit_map));
            m
        });
        agents.insert(
            "plan".to_string(),
            Info {
                name: "plan".into(),
                description: Some("Plan mode. Disallows all edit tools.".into()),
                permission: oc_permission::merge_rulesets(&[
                    defaults.clone(),
                    oc_permission::from_config_value(&plan_config),
                    user_ruleset.clone(),
                ]),
                mode: Mode::Primary,
                native: true,
                ..Default::default()
            },
        );
    }

    // general
    agents.insert(
        "general".to_string(),
        Info {
            name: "general".into(),
            description: Some("General-purpose agent for researching complex questions and executing multi-step tasks. Use this agent to execute multiple units of work in parallel.".into()),
            permission: oc_permission::merge_rulesets(&[
                defaults.clone(),
                oc_permission::from_config_value(&serde_json::json!({"todowrite":"deny"})),
                user_ruleset.clone(),
            ]),
            options: Default::default(),
            mode: Mode::Subagent,
            native: true,
            ..Default::default()
        },
    );

    // explore
    {
        let tmp_glob = format!("{}/{}", tmp_dir.display(), "*").replace('\\', "/");
        let mut ro_ext = serde_json::Map::new();
        ro_ext.insert("*".into(), serde_json::json!("ask"));
        ro_ext.insert(
            format!("{}/tool-output/*", data_dir.display()).replace('\\', "/"),
            serde_json::json!("allow"),
        );
        ro_ext.insert(tmp_glob, serde_json::json!("allow"));
        let explore_config = serde_json::Value::Object({
            let mut m = serde_json::Map::new();
            m.insert("*".into(), serde_json::json!("deny"));
            for key in [
                "grep",
                "glob",
                "list",
                "bash",
                "webfetch",
                "websearch",
                "read",
            ] {
                m.insert(key.into(), serde_json::json!("allow"));
            }
            m.insert(
                "external_directory".into(),
                serde_json::Value::Object(ro_ext),
            );
            m
        });
        agents.insert(
            "explore".to_string(),
            Info {
                name: "explore".into(),
                permission: oc_permission::merge_rulesets(&[
                    defaults.clone(),
                    oc_permission::from_config_value(&explore_config),
                    user_ruleset.clone(),
                ]),
                description: Some("Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns (eg. \"src/components/**/*.tsx\"), search code for keywords (eg. \"API endpoints\"), or answer questions about the codebase (eg. \"how do API endpoints work?\"). When calling this agent, specify the desired thoroughness level: \"quick\" for basic searches, \"medium\" for moderate exploration, or \"very thorough\" for comprehensive analysis across multiple locations and naming conventions.".into()),
                prompt: Some(PROMPT_EXPLORE.to_string()),
                options: Default::default(),
                mode: Mode::Subagent,
                native: true,
                ..Default::default()
            },
        );
    }

    // compaction / title / summary
    for (name, mode, hidden, prompt, temp) in [
        ("compaction", Mode::Primary, true, PROMPT_COMPACTION, None),
        ("title", Mode::Primary, true, PROMPT_TITLE, Some(0.5)),
        ("summary", Mode::Primary, true, PROMPT_SUMMARY, None),
    ] {
        agents.insert(
            name.to_string(),
            Info {
                name: name.into(),
                mode,
                native: true,
                hidden,
                temperature: temp,
                prompt: Some(prompt.to_string()),
                permission: oc_permission::merge_rulesets(&[
                    defaults.clone(),
                    oc_permission::from_config_value(&serde_json::json!({"*":"deny"})),
                    user_ruleset.clone(),
                ]),
                options: Default::default(),
                ..Default::default()
            },
        );
    }

    // config overrides (agent.ts:267-294)
    for (key, value) in config_overrides {
        if value.disable == Some(true) {
            agents.remove(key);
            continue;
        }
        let item = agents.entry(key.clone()).or_insert_with(|| Info {
            name: key.clone(),
            mode: Mode::All,
            permission: oc_permission::merge_rulesets(&[defaults.clone(), user_ruleset.clone()]),
            native: false,
            ..Default::default()
        });
        if let Some(model_str) = &value.model {
            let parsed = oc_provider::parse_model(model_str);
            item.model = Some(ModelRef {
                provider_id: parsed.provider_id,
                model_id: parsed.model_id,
            });
        }
        item.variant = value.variant.clone().or(item.variant.take());
        item.prompt = value.prompt.clone().or(item.prompt.take());
        item.description = value.description.clone().or(item.description.take());
        item.temperature = value.temperature.or(item.temperature.take());
        item.top_p = value.top_p.or(item.top_p.take());
        if let Some(mode_str) = &value.mode {
            item.mode = Mode::from_config_str(mode_str);
        }
        item.color = value.color.clone().or(item.color.take());
        if let Some(h) = value.hidden {
            item.hidden = h;
        }
        if let Some(name_override) = &value.name {
            item.name = name_override.clone();
        }
        item.steps = value.steps.or(item.steps.take());
        if let Some(options) = &value.options {
            for (k, v) in options {
                item.options.insert(k.clone(), v.clone());
            }
        }
        if let Some(perm_config) = &value.permission {
            item.permission
                .extend(oc_permission::from_config_value(perm_config));
        }
    }

    // Truncate.GLOB allow unless explicitly denied (agent.ts:297-310)
    let truncation_glob = format!("{}/tool-output/*", data_dir.display()).replace('\\', "/");
    let names: Vec<String> = agents.keys().cloned().collect();
    for name in names {
        let agent = agents.get_mut(&name).unwrap();
        let explicit_denied = agent.permission.iter().any(|r| {
            r.permission == "external_directory"
                && r.action == Action::Deny
                && r.pattern == truncation_glob
        });
        if !explicit_denied {
            let mut ext_dir = serde_json::Map::new();
            ext_dir.insert(truncation_glob.clone(), serde_json::json!("allow"));
            let allow_rule = oc_permission::from_config_value(
                &serde_json::json!({"external_directory": ext_dir}),
            );
            agent.permission.extend(allow_rule);
        }
    }

    agents
}

/// Ported dari agent.ts:312-344 — get/list/defaultInfo/defaultAgent.
pub struct AgentRegistry {
    pub agents: BTreeMap<String, Info>,
}

impl AgentRegistry {
    pub fn new(
        data_dir: &std::path::Path,
        tmp_dir: &std::path::Path,
        user_ruleset: Vec<oc_permission::Rule>,
        config_overrides: &BTreeMap<String, AgentConfigOverride>,
    ) -> Self {
        AgentRegistry {
            agents: build_agents(data_dir, tmp_dir, user_ruleset, config_overrides),
        }
    }

    pub fn get(&self, agent_name: &str) -> Option<&Info> {
        self.agents.get(agent_name)
    }

    /// Sorted by default-first/name asc.
    pub fn list(&self, default_agent: Option<&str>) -> Vec<&Info> {
        let target = default_agent.unwrap_or("build");
        let mut items: Vec<&Info> = self.agents.values().collect();
        items.sort_by(|a, b| {
            let a_match = a.name == target;
            let b_match = b.name == target;
            b_match.cmp(&a_match).then_with(|| a.name.cmp(&b.name))
        });
        items
    }

    pub fn default_info(&self, config_default_agent: Option<&str>) -> Result<&Info, String> {
        if let Some(name) = config_default_agent {
            let agent = self
                .agents
                .get(name)
                .ok_or_else(|| format!("default agent \"{name}\" not found"))?;
            if agent.mode == Mode::Subagent {
                return Err(format!("default agent \"{name}\" is a subagent"));
            }
            if agent.hidden {
                return Err(format!("default agent \"{name}\" is hidden"));
            }
            return Ok(agent);
        }
        self.agents
            .values()
            .find(|a| a.mode != Mode::Subagent && !a.hidden)
            .ok_or_else(|| "no primary visible agent found".to_string())
    }

    pub fn default_agent(&self, config_default_agent: Option<&str>) -> Result<String, String> {
        self.default_info(config_default_agent)
            .map(|a| a.name.clone())
    }
}
