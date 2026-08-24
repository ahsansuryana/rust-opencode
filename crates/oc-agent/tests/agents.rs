//! Test oc-agent (Sprint 8): built-in agents, permission, config override.

use std::collections::BTreeMap;

use oc_agent::registry::AgentRegistry;
use oc_agent::{AgentConfigOverride, Mode};
use oc_permission::Action;

fn make_registry(overrides: &BTreeMap<String, AgentConfigOverride>) -> AgentRegistry {
    let data = std::path::Path::new("/tmp/oc-agent-test/data");
    let tmp = std::path::Path::new("/tmp/oc-agent-test/tmp");
    AgentRegistry::new(data, tmp, vec![], overrides)
}

#[test]
fn builtin_agents_exist_and_have_correct_modes() {
    let registry = make_registry(&Default::default());
    for name in [
        "build",
        "plan",
        "general",
        "explore",
        "compaction",
        "title",
        "summary",
    ] {
        assert!(registry.get(name).is_some(), "{name} harus ada");
    }
    assert_eq!(registry.get("build").unwrap().mode, Mode::Primary);
    assert_eq!(registry.get("general").unwrap().mode, Mode::Subagent);
    assert!(registry.get("compaction").unwrap().hidden);
    assert_eq!(registry.get("title").unwrap().temperature, Some(0.5));
}

#[test]
fn explore_agent_denies_edit_allows_read_only_tools() {
    let registry = make_registry(&Default::default());
    let explore = registry.get("explore").unwrap();

    // prompt harus dari file verbatim
    assert_eq!(
        explore.prompt.as_deref(),
        Some(include_str!("../prompts/explore.txt"))
    );

    // permission: gunakan evaluate() seperti runtime
    let rules = &explore.permission;
    let eval = |perm: &str| {
        let rule = oc_permission::evaluate(perm, "*", std::slice::from_ref(rules));
        rule.action
    };
    assert_eq!(eval("grep"), Action::Allow);
    assert_eq!(eval("read"), Action::Allow);
    // "*" deny menutup edit
    assert_eq!(eval("edit"), Action::Deny);
}

#[test]
fn plan_agent_denies_edit() {
    let registry = make_registry(&Default::default());
    let plan = registry.get("plan").unwrap();
    let edit_rule = plan
        .permission
        .iter()
        .rev()
        .find(|r| r.permission == "edit")
        .expect("plan harus punya rule edit");
    assert_eq!(edit_rule.action, Action::Deny);
}

#[test]
fn default_agent_is_build_without_config() {
    let registry = make_registry(&Default::default());
    assert_eq!(registry.default_agent(None).unwrap(), "build");
}

#[test]
fn default_agent_config_override() {
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "custom".to_string(),
        AgentConfigOverride {
            mode: Some("primary".into()),
            description: Some("My custom agent".into()),
            ..Default::default()
        },
    );
    overrides.insert(
        "build".to_string(),
        AgentConfigOverride {
            disable: Some(true),
            ..Default::default()
        },
    );
    let registry = make_registry(&overrides);

    // build disabled → default adalah custom
    let result = registry.default_agent(None).unwrap();
    assert_eq!(result, "custom");

    // custom agent ada dan primary
    let info = registry.get("custom").unwrap();
    assert_eq!(info.mode, Mode::Primary);
    assert!(!info.native);
}

#[test]
fn config_override_can_change_model_and_prompt() {
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "build".to_string(),
        AgentConfigOverride {
            model: Some("openai/gpt-5".into()),
            prompt: Some("Custom system prompt.".into()),
            temperature: Some(0.7),
            ..Default::default()
        },
    );
    let registry = make_registry(&overrides);
    let build = registry.get("build").unwrap();
    assert_eq!(
        build.model.as_ref().map(|m| m.provider_id.as_str()),
        Some("openai")
    );
    assert_eq!(
        build.model.as_ref().map(|m| m.model_id.as_str()),
        Some("gpt-5")
    );
    assert_eq!(build.prompt.as_deref(), Some("Custom system prompt."));
    assert_eq!(build.temperature, Some(0.7));
}

#[test]
fn prompts_are_verbatim_from_source_files() {
    assert!(oc_agent::registry::PROMPT_COMPACTION.contains("conversation"));
    assert!(oc_agent::registry::PROMPT_EXPLORE.contains("thoroughness"));
    assert!(oc_agent::registry::PROMPT_TITLE.contains("title"));
    assert!(oc_agent::registry::PROMPT_SUMMARY.contains("summary"));
    assert!(oc_agent::registry::PROMPT_GENERATE.len() > 100);
}
