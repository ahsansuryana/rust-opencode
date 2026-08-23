//! Test integrasi loader config (Sprint 1).
//! Membuktikan precedence global → custom → project → .opencode dirs →
//! OPENCODE_CONFIG_CONTENT, plus transformasi turunan (tools→permission,
//! mode→agent, autoshare, compaction flags, username, managed dir, migrasi
//! legacy TOML, seed global config).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use oc_config::config::{ConfigHandle, InstanceContext};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn set_env(key: &str, value: Option<&str>) {
    match value {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

struct EnvGuard;
impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in [
            "OPENCODE_CONFIG",
            "OPENCODE_CONFIG_CONTENT",
            "OPENCODE_CONFIG_DIR",
            "OPENCODE_PERMISSION",
            "OPENCODE_TEST_MANAGED_CONFIG_DIR",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_STATE_HOME",
        ] {
            set_env(key, None);
        }
        oc_global::flag::reset_for_tests();
        oc_global::reset_for_tests();
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oc-loader-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn setup_xdg(root: &Path) {
    set_env("HOME", Some(root.to_str().unwrap()));
    set_env("USERPROFILE", Some(root.to_str().unwrap()));
    set_env("XDG_CONFIG_HOME", None);
    set_env("OPENCODE_CONFIG_DIR", None);
    oc_global::reset_for_tests();
}

fn ctx_of(directory: &Path) -> InstanceContext {
    InstanceContext {
        directory: directory.to_path_buf(),
        worktree: None,
    }
}

#[test]
fn precedence_global_custom_project_dot_opencode_content() {
    let _guard_env = env_lock();
    let _guard = EnvGuard;
    let root = temp_dir("precedence");
    setup_xdg(&root);

    // 1. GLOBAL (config.json di XDG)
    let global_dir = root.join(".config/opencode");
    std::fs::create_dir_all(&global_dir).unwrap();
    std::fs::write(
        global_dir.join("config.json"),
        r#"{"model":"global/model","username":"global-user","instructions":["a.md"],"share":"disabled"}"#,
    )
    .unwrap();

    // 2. CUSTOM (OPENCODE_CONFIG) menimpa model
    let custom_file = root.join("custom.json");
    std::fs::write(&custom_file, r#"{"model":"custom/model"}"#).unwrap();
    set_env("OPENCODE_CONFIG", Some(custom_file.to_str().unwrap()));

    // 3. PROJECT: rantai dua level; inner menimpa outer
    let project = root.join("worktree/deep");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        root.join("worktree/opencode.json"),
        r#"{"small_model":"outer/model","instructions":["b.md"]}"#,
    )
    .unwrap();
    std::fs::write(
        project.join("opencode.json"),
        r#"{ "small_model":"inner/model" }"#,
    )
    .unwrap();

    // .opencode dirs di-load setelah project files
    std::fs::create_dir_all(project.join(".opencode")).unwrap();
    std::fs::write(
        project.join(".opencode/opencode.json"),
        r#"{"default_agent":"from-dot-opencode"}"#,
    )
    .unwrap();

    // 4. CONTENT paling akhir menimpa model
    set_env(
        "OPENCODE_CONFIG_CONTENT",
        Some(r#"{"model":"content/model"}"#),
    );

    let handle = ConfigHandle::new();
    let state = handle.load_instance_state(&ctx_of(&project)).unwrap();
    let cfg = &state.config;

    assert_eq!(cfg["model"], "content/model");
    assert_eq!(cfg["small_model"], "inner/model");
    assert_eq!(cfg["username"], "global-user");
    // instructions concat + dedupe first-seen: a.md (global), b.md (outer)
    assert_eq!(cfg["instructions"], serde_json::json!(["a.md", "b.md"]));
    assert_eq!(cfg["share"], "disabled");
    assert_eq!(cfg["default_agent"], "from-dot-opencode");

    // directories(): global config dir, worktree/.opencode, home/.opencode
    assert!(state.directories.contains(&global_dir));
    assert!(state.directories.contains(&project.join(".opencode")));

    // ensureGitignore membuat .gitignore di tiap direktori config
    assert!(project.join(".opencode/.gitignore").exists());
}

#[test]
fn seed_and_schema_injection_write_backs() {
    let _guard_env = env_lock();
    let _guard = EnvGuard;
    let root = temp_dir("seed-schema");
    setup_xdg(&root);

    let global_dir = root.join(".config/opencode");
    std::fs::create_dir_all(&global_dir).unwrap();

    let handle = ConfigHandle::new();
    handle.get_global();
    // Seed: file kandidat pertama (opencode.jsonc) dibuat berisi $schema
    let seeded = global_dir.join("opencode.jsonc");
    let text = std::fs::read_to_string(&seeded).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text).unwrap(),
        serde_json::json!({"$schema":"https://opencode.ai/config.json"})
    );

    // $schema injection pada project file tanpa $schema
    let project = root.join("proj");
    std::fs::create_dir_all(&project).unwrap();
    let file = project.join("opencode.json");
    std::fs::write(&file, "{\n  \"shell\": \"/bin/zsh\"\n}").unwrap();

    let handle2 = ConfigHandle::new();
    handle2.load_instance_state(&ctx_of(&project)).unwrap();
    let updated = std::fs::read_to_string(&file).unwrap();
    assert!(updated.starts_with("{\n  \"$schema\": \"https://opencode.ai/config.json\","));
    assert!(updated.contains("\"shell\": \"/bin/zsh\""));
}

#[test]
fn legacy_toml_migration_runs_once() {
    let _guard_env = env_lock();
    let _guard = EnvGuard;
    let root = temp_dir("legacy-toml");
    setup_xdg(&root);

    let global_dir = root.join(".config/opencode");
    std::fs::create_dir_all(&global_dir).unwrap();
    std::fs::write(
        global_dir.join("config"),
        "provider = \"anthropic\"\nmodel = \"claude-3\"\ntheme = \"dark\"\n",
    )
    .unwrap();

    let handle = ConfigHandle::new();
    let global = handle.get_global();
    assert_eq!(global["model"], "anthropic/claude-3");
    assert_eq!(global["theme"], "dark");
    assert_eq!(global["$schema"], "https://opencode.ai/config.json");
    // legacy file terhapus, config.json tertulis
    assert!(!global_dir.join("config").exists());
    assert!(global_dir.join("config.json").exists());
}

#[test]
fn derived_transformations_match_ts_pipeline() {
    let _guard_env = env_lock();
    let _guard = EnvGuard;
    let root = temp_dir("derived");
    setup_xdg(&root);

    let global_dir = root.join(".config/opencode");
    std::fs::create_dir_all(&global_dir).unwrap();
    std::fs::write(
        global_dir.join("config.json"),
        r#"{
          "mode": {"plan": {"prompt": "planning"}},
          "tools": {"write": false, "webfetch": true},
          "autoshare": true,
          "subagent_depth": 2
        }"#,
    )
    .unwrap();

    set_env(
        "OPENCODE_PERMISSION",
        Some(r#"{"bash": {"rm*": "deny"}, "edit": "ask"}"#),
    );
    set_env("OPENCODE_DISABLE_AUTOCOMPACT", Some("1"));

    let project = root.join("proj");
    std::fs::create_dir_all(&project).unwrap();

    let handle = ConfigHandle::new();
    let info: oc_config::v1::config::Info = parse_info(
        handle
            .load_instance_state(&ctx_of(&project))
            .unwrap()
            .config,
    );

    // mode → agent promotion dengan mode:"primary"
    let plan = info.agent.as_ref().unwrap().get("plan").unwrap();
    assert_eq!(plan.prompt.as_deref(), Some("planning"));
    assert_eq!(plan.mode, Some(oc_config::v1::agent::AgentMode::Primary));

    // tools→permission: write=false → edit=deny; webfetch=true → allow;
    // lalu OPENCODE_PERMISSION menimpa: edit=ask, bash rule object
    let permission = info.permission.unwrap();
    use oc_config::v1::permission::PermissionRule;
    let edit_rule = permission.object.edit.as_ref().unwrap();
    assert_eq!(
        serde_json::to_value(edit_rule).unwrap(),
        serde_json::json!("ask")
    );
    assert!(matches!(edit_rule, PermissionRule::Action(_)));
    let webfetch = permission.object.webfetch.unwrap();
    assert_eq!(
        serde_json::to_value(webfetch).unwrap(),
        serde_json::json!("allow")
    );
    // `bash` adalah key dikenal → tersimpan di field bertipe, bukan rest
    let bash_rule = permission.object.bash.as_ref().unwrap();
    assert_eq!(
        serde_json::to_value(bash_rule).unwrap(),
        serde_json::json!({"rm*": "deny"})
    );

    // autoshare=true & share kosong → share="auto"
    assert_eq!(
        serde_json::to_value(info.share).unwrap(),
        serde_json::json!("auto")
    );

    // compaction.auto=false karena flag
    assert_eq!(info.compaction.unwrap().auto, Some(false));

    // username default dari sistem
    assert!(info.username.is_some());

    // subagent_depth non-negative int lolos validasi
    assert_eq!(info.subagent_depth, Some(2));
}

fn parse_info(value: serde_json::Value) -> oc_config::v1::config::Info {
    oc_config::parse::schema_decode::<oc_config::v1::config::Info>(value, "").unwrap()
}

#[test]
fn managed_config_dir_loaded_via_test_env() {
    let _guard_env = env_lock();
    let _guard = EnvGuard;
    let root = temp_dir("managed");
    setup_xdg(&root);

    let managed_dir = root.join("managed-config");
    std::fs::create_dir_all(managed_dir.join("nested")).unwrap();
    std::fs::write(
        managed_dir.join("opencode.json"),
        r#"{"model":"managed/model"}"#,
    )
    .unwrap();
    // OPENCODE_TEST_MANAGED_CONFIG_DIR mengganti systemManagedConfigDir()
    set_env(
        "OPENCODE_TEST_MANAGED_CONFIG_DIR",
        Some(managed_dir.to_str().unwrap()),
    );

    let project = root.join("proj");
    std::fs::create_dir_all(&project).unwrap();

    let handle = ConfigHandle::new();
    let state = handle.load_instance_state(&ctx_of(&project)).unwrap();
    assert_eq!(state.config["model"], "managed/model");
}
