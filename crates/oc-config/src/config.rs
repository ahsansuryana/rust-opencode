//! Ported from: packages/opencode/src/config/config.ts
//!
//! PORT SEBAGIAN deterministik (lihat DEVIATIONS.md): jalur remote well-known,
//! org console, npm install, discovery agent/command/plugin markdown, dan
//! write-back update/updateGlobal ditunda ke sprint berikutnya.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oc_global::flag;
use oc_global::global;
use serde_json::{Map, Value};

use crate::managed;
use crate::parse;
use crate::paths as config_paths;
use crate::v1::config::Info;
use crate::v1::error::{InvalidError, JsonError};
use crate::variable::{self, MissingPolicy, ParseSource, SubstituteInput};

/// Error agregat lapisan config (padanan gabungan JsonError/InvalidError/IO).
#[derive(Debug)]
pub enum ConfigLoadError {
    Io(io::Error),
    Json(JsonError),
    Invalid(InvalidError),
}

impl From<io::Error> for ConfigLoadError {
    fn from(value: io::Error) -> Self {
        ConfigLoadError::Io(value)
    }
}

impl From<JsonError> for ConfigLoadError {
    fn from(value: JsonError) -> Self {
        ConfigLoadError::Json(value)
    }
}

impl From<InvalidError> for ConfigLoadError {
    fn from(value: InvalidError) -> Self {
        ConfigLoadError::Invalid(value)
    }
}

impl From<serde_json::Error> for ConfigLoadError {
    fn from(value: serde_json::Error) -> Self {
        ConfigLoadError::Io(io::Error::other(value.to_string()))
    }
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigLoadError::Io(error) => write!(f, "io error: {error}"),
            ConfigLoadError::Json(error) => write!(f, "{error}"),
            ConfigLoadError::Invalid(error) => write!(f, "{error}"),
        }
    }
}

/// Padanan minimal InstanceContext dari project/instance-context.ts
/// (field yang dipakai jalur config; versi kanonik menyusul di crate project).
#[derive(Debug, Clone)]
pub struct InstanceContext {
    pub directory: PathBuf,
    pub worktree: Option<PathBuf>,
}

/// Ported from: packages/opencode/src/config/config.ts:41-43 (mergeConfig)
/// = remeda mergeDeep: plain-object kiri-kanan direkursi, sisanya source menang.
pub fn merge_deep(target: &Value, source: &Value) -> Value {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            let mut out = target_map.clone();
            for (key, source_value) in source_map {
                match target_map.get(key) {
                    Some(target_value) if target_value.is_object() && source_value.is_object() => {
                        let merged = merge_deep(target_value, source_value);
                        out.insert(key.clone(), merged);
                    }
                    _ => {
                        out.insert(key.clone(), source_value.clone());
                    }
                }
            }
            Value::Object(out)
        }
        _ => source.clone(),
    }
}

/// Ported from: packages/opencode/src/config/config.ts:45-51 (mergeConfigConcatArrays)
pub fn merge_deep_concat_arrays(target: &Value, source: &Value) -> Value {
    let merged = merge_deep(target, source);
    let t_instructions = target
        .get("instructions")
        .and_then(Value::as_array)
        .cloned();
    let s_instructions = source
        .get("instructions")
        .and_then(Value::as_array)
        .cloned();
    if let (Some(t), Some(s)) = (t_instructions, s_instructions) {
        // Array.from(new Set([...target, ...source])) — urutan first-seen
        let mut seen: Vec<&Value> = Vec::new();
        let mut combined = Vec::with_capacity(t.len() + s.len());
        for item in t.iter().chain(s.iter()) {
            if !seen.contains(&item) {
                seen.push(item);
                combined.push(item.clone());
            }
        }
        if let Value::Object(map) = &merged {
            let mut map = map.clone();
            map.insert("instructions".to_string(), Value::Array(combined));
            return Value::Object(map);
        }
    }
    merged
}

/// Ported from: packages/opencode/src/config/config.ts:53-62 (normalizeLoadedConfig)
pub fn normalize_loaded_config(data: Value) -> Value {
    let Value::Object(map) = data else {
        return data;
    };
    let had_legacy =
        map.contains_key("theme") || map.contains_key("keybinds") || map.contains_key("tui");
    if !had_legacy {
        return Value::Object(map);
    }
    let mut copy = map;
    copy.remove("theme");
    copy.remove("keybinds");
    copy.remove("tui");
    Value::Object(copy)
}

/// Ported from: packages/opencode/src/config/config.ts:139-147 (globalConfigFile)
pub fn global_config_file() -> PathBuf {
    let paths = global::path();
    let candidates = ["opencode.jsonc", "opencode.json", "config.json"];
    for file in candidates {
        let file_path = paths.config.join(file);
        if file_path.exists() {
            return file_path;
        }
    }
    paths.config.join(candidates[0])
}

/// Meniru `text.replace(/^\s*\{/, '{\n  "$schema": "...",')`
fn insert_schema_line(text: &str) -> String {
    let Some((start, ch)) = text.char_indices().find(|(_, c)| !c.is_whitespace()) else {
        return text.to_string();
    };
    let _ = ch;
    if text.as_bytes().get(start) != Some(&b'{') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 64);
    out.push_str(&text[..=start]);
    out.push_str("\n  \"$schema\": \"https://opencode.ai/config.json\",");
    out.push_str(&text[start + 1..]);
    out
}

/// Ported from: packages/opencode/src/config/config.ts:213-237 (loadConfig)
/// Catatan: `resolveLoadedPlugins` ditunda (plugin subsystem).
pub fn load_config(
    text: &str,
    options: &ParseSource,
    env: Option<&BTreeMap<String, String>>,
) -> Result<Value, ConfigLoadError> {
    let source = match options {
        ParseSource::Path { path } => path.to_string_lossy().into_owned(),
        ParseSource::Virtual { source, .. } => source.clone(),
    };
    let expanded = variable::substitute(&SubstituteInput {
        parse_source: options,
        text: text.to_string(),
        missing: MissingPolicy::Error,
        env,
    })?;
    let parsed = parse::jsonc(&expanded, &source)?;
    let data = normalize_loaded_config(parsed);

    // ConfigParse.schema(ConfigV1.Info, ...) → validasi + hasil decode.
    // Round-trip Info meniru perilaku decode TS: excess property dibuang,
    // agent/permission dinormalisasi.
    let info: Info = parse::schema_decode::<Info>(data, &source).map_err(ConfigLoadError::from)?;
    let mut decoded = serde_json::to_value(&info)?;

    let ParseSource::Path { path } = options else {
        return Ok(decoded);
    };

    // $schema injection + write-back (config.ts:231-235); hanya mode "path".
    if info.schema.is_none() {
        if let Value::Object(map) = &mut decoded {
            map.insert(
                "$schema".to_string(),
                Value::String("https://opencode.ai/config.json".to_string()),
            );
        }
        let updated = insert_schema_line(text);
        let _: Result<(), _> = std::fs::write(path, updated);
    }
    Ok(decoded)
}

/// Ported from: packages/opencode/src/config/config.ts:185, 239-244 (readConfigFile/loadFile)
pub fn load_file(
    filepath: &Path,
    env: Option<&BTreeMap<String, String>>,
) -> Result<Value, ConfigLoadError> {
    tracing::info!(path = %filepath.display(), "loading");
    let text = crate::fs_util::read_file_string_safe(filepath)?;
    let Some(text) = text else {
        return Ok(empty_config());
    };
    if text.is_empty() {
        return Ok(empty_config());
    }
    load_config(
        &text,
        &ParseSource::Path {
            path: filepath.to_path_buf(),
        },
        env,
    )
}

fn empty_config() -> Value {
    Value::Object(Map::new())
}

/// Ported from: packages/opencode/src/config/config.ts:246-279 (loadGlobal)
pub fn load_global(env: Option<&BTreeMap<String, String>>) -> Result<Value, ConfigLoadError> {
    let paths = global::path();
    let mut result = empty_config();

    // Seed default config untuk editor completion (config.ts:250-257)
    if flag::open_code_config().is_none()
        && flag::open_code_config_dir().is_none()
        && flag::open_code_config_content().is_none()
    {
        let file = global_config_file();
        if !file.exists() {
            let seed = serde_json::to_string_pretty(&serde_json::json!({
                "$schema": "https://opencode.ai/config.json"
            }))
            .unwrap_or_else(|_| "{}".to_string());
            if let Some(parent) = file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _: Result<(), _> = std::fs::write(&file, seed);
        }
    }

    for file in ["config.json", "opencode.json", "opencode.jsonc"] {
        let file_path = paths.config.join(file);
        result = merge_deep(&result, &load_file(&file_path, env)?);
    }

    // Migrasi legacy TOML `config` (config.ts:262-276); seluruh blok best-effort.
    let legacy = paths.config.join("config");
    if legacy.exists() {
        migrate_legacy_toml(&legacy, &mut result);
    }

    Ok(result)
}

fn migrate_legacy_toml(legacy: &Path, result: &mut Value) {
    let mut migration = || -> Result<(), Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(legacy)?;
        let table: toml::Table = match text.parse() {
            Ok(table) => table,
            Err(_) => return Ok(()),
        };
        let json = toml_to_json(&toml::Value::Table(table));
        let Value::Object(obj) = json else {
            return Ok(());
        };
        // const { provider, model, ...rest } = mod.default
        let provider = obj.get("provider").cloned();
        let model = obj.get("model").cloned();
        let mut rest = obj.clone();
        rest.remove("provider");
        rest.remove("model");

        // if (provider && model) result.model = `${provider}/${model}`
        let provider_str = provider
            .as_ref()
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        let model_str = model
            .as_ref()
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        if let (Some(provider_str), Some(model_str)) = (provider_str, model_str) {
            if let Value::Object(map) = result {
                map.insert(
                    "model".to_string(),
                    Value::String(format!("{provider_str}/{model_str}")),
                );
            }
        }
        if let Value::Object(map) = result {
            map.insert(
                "$schema".to_string(),
                Value::String("https://opencode.ai/config.json".to_string()),
            );
        }
        *result = merge_deep(result, &Value::Object(rest));

        // writeFile(path.join(Global.Path.config, "config.json"), stringify(result))
        let config_json = global::path().config.join("config.json");
        let serialized = serde_json::to_string_pretty(result)?;
        std::fs::write(config_json, serialized)?;
        let _ = std::fs::remove_file(legacy);
        Ok(())
    };
    let _ = migration();
}

fn toml_to_json(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => {
            serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number)
        }
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(items) => Value::Array(items.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let mut map = Map::new();
            for (key, value) in table {
                map.insert(key.clone(), toml_to_json(value));
            }
            Value::Object(map)
        }
    }
}

/// Ported from: packages/opencode/src/config/config.ts:295-312 (ensureGitignore)
pub fn ensure_gitignore(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        let content = [
            "node_modules",
            "package.json",
            "package-lock.json",
            "bun.lock",
            ".gitignore",
        ]
        .join("\n");
        match std::fs::write(&gitignore, content) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// State hasil loadInstanceState (config.ts:117-122; deps/consoleState ditunda).
#[derive(Debug, Clone)]
pub struct ConfigState {
    pub config: Value,
    pub directories: Vec<PathBuf>,
}

/// Service padanan Interface config.ts:124-133 (subset yang diport sprint ini).
pub struct ConfigHandle {
    cached_global: Mutex<Option<Value>>,
    instance_cache: Mutex<Option<ConfigState>>,
}

impl Default for ConfigHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigHandle {
    pub fn new() -> Self {
        ConfigHandle {
            cached_global: Mutex::new(None),
            instance_cache: Mutex::new(None),
        }
    }

    /// Ported from: config.ts:281-293 (cachedGlobal + getGlobal) — error
    /// dilog lalu fallback `{}` yang ikut ter-cache.
    pub fn get_global(&self) -> Value {
        let mut cache = self.cached_global.lock().unwrap();
        if let Some(cached) = cache.as_ref() {
            return cached.clone();
        }
        let loaded = match load_global(None) {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(error = %error, "failed to load global config, using defaults");
                empty_config()
            }
        };
        *cache = Some(loaded.clone());
        loaded
    }

    /// Ported from: config.ts:633-635 (invalidate)
    pub fn invalidate(&self) {
        *self.cached_global.lock().unwrap() = None;
        *self.instance_cache.lock().unwrap() = None;
    }

    /// Ported from: config.ts:314-598 (loadInstanceState), subset deterministik.
    pub fn load_instance_state(
        &self,
        ctx: &InstanceContext,
    ) -> Result<ConfigState, ConfigLoadError> {
        let mut result = empty_config();

        // Jalur well-known auth remote (config.ts:356-396): butuh Auth — ditunda.

        let global_config = self.get_global();
        result = merge_deep_concat_arrays(&result, &global_config);

        if let Some(custom) = flag::open_code_config() {
            let next = load_file(Path::new(&custom), None)?;
            result = merge_deep_concat_arrays(&result, &next);
            tracing::debug!(path = %custom, "loaded custom config");
        }

        if !flag::open_code_disable_project_config() {
            let files = config_paths::files("opencode", &ctx.directory, ctx.worktree.as_deref())?;
            for file in files {
                let next = load_file(&file, None)?;
                result = merge_deep_concat_arrays(&result, &next);
            }
        }

        // result.agent ||= {}; result.mode ||= {}; result.plugin ||= []
        ensure_default_objects(&mut result);

        let directories = config_paths::directories(&ctx.directory, ctx.worktree.as_deref())?;

        if flag::open_code_config_dir().is_some() {
            tracing::debug!("loading config from OPENCODE_CONFIG_DIR");
        }

        for dir in &directories {
            let is_custom_dir = flag::open_code_config_dir()
                .as_ref()
                .map(|custom| Path::new(custom.as_str()) == dir.as_path())
                .unwrap_or(false);
            if dir.ends_with(".opencode") || is_custom_dir {
                for file in ["opencode.json", "opencode.jsonc"] {
                    let source = dir.join(file);
                    let next = load_file(&source, None)?;
                    result = merge_deep_concat_arrays(&result, &next);
                    ensure_default_objects(&mut result);
                }
            }

            ensure_gitignore(dir)?;
            // npm dependency install fork (config.ts:438-457): ditunda.
            // ConfigCommand.load / ConfigAgent.load / loadMode / ConfigPlugin.load:
            // ditunda (markdown/plugin discovery).
        }

        if let Ok(content) = std::env::var("OPENCODE_CONFIG_CONTENT") {
            let source = "OPENCODE_CONFIG_CONTENT";
            let next = load_config(
                &content,
                &ParseSource::Virtual {
                    source: source.to_string(),
                    dir: ctx.directory.clone(),
                },
                None,
            )?;
            result = merge_deep_concat_arrays(&result, &next);
            tracing::debug!("loaded custom config from OPENCODE_CONFIG_CONTENT");
        }

        // Org console remote config (config.ts:478-514): butuh Account — ditunda.

        // Managed settings file-based (config.ts:516-522)
        let managed_dir = managed::managed_config_dir();
        if managed_dir.exists() {
            for file in ["opencode.json", "opencode.jsonc"] {
                let source = managed_dir.join(file);
                let next = load_file(&source, None)?;
                result = merge_deep_concat_arrays(&result, &next);
            }
        }

        // macOS managed preferences (.mobileconfig MDM) — config.ts:524-534
        if let Some(preferences) = managed::read_managed_preferences() {
            let dir = Path::new(&preferences.source)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            let next = load_config(
                &preferences.text,
                &ParseSource::Virtual {
                    source: preferences.source.clone(),
                    dir,
                },
                None,
            )?;
            result = merge_deep_concat_arrays(&result, &next);
        }

        // mode → agent promotion (config.ts:536-543)
        promote_mode_to_agent(&mut result);

        // OPENCODE_PERMISSION env (config.ts:545-551)
        if let Some(raw) = flag::open_code_permission() {
            match serde_json::from_str::<Value>(&raw) {
                Ok(parsed) => {
                    let existing = result
                        .get("permission")
                        .cloned()
                        .unwrap_or_else(empty_config);
                    let permission = merge_deep(&existing, &parsed);
                    if let Value::Object(map) = &mut result {
                        map.insert("permission".to_string(), permission);
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        err = %error,
                        "OPENCODE_PERMISSION contains invalid JSON, skipping"
                    );
                }
            }
        }

        // tools → permission conversion (config.ts:553-564)
        if let Some(tools) = result.get("tools").and_then(Value::as_object).cloned() {
            let mut perms = Map::new();
            for (tool, enabled) in tools {
                let enabled = enabled.as_bool().unwrap_or(false);
                let action = if enabled { "allow" } else { "deny" };
                if tool == "write" || tool == "edit" || tool == "patch" {
                    perms.insert("edit".to_string(), Value::String(action.to_string()));
                    continue;
                }
                perms.insert(tool, Value::String(action.to_string()));
            }
            let existing = result
                .get("permission")
                .cloned()
                .unwrap_or_else(empty_config);
            let permission = merge_deep(&Value::Object(perms), &existing);
            if let Value::Object(map) = &mut result {
                map.insert("permission".to_string(), permission);
            }
        }

        // username default (config.ts:566-573): if (!result.username)
        let username_missing = match result.get("username") {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            Some(_) => false,
        };
        if username_missing {
            let system_username = whoami::username();
            let username = if system_username.is_empty() {
                "user".to_string()
            } else {
                system_username
            };
            if let Value::Object(map) = &mut result {
                map.insert("username".to_string(), Value::String(username));
            }
        }

        // autoshare → share (config.ts:575-577)
        if result.get("autoshare") == Some(&Value::Bool(true)) && result.get("share").is_none() {
            if let Value::Object(map) = &mut result {
                map.insert("share".to_string(), Value::String("auto".to_string()));
            }
        }

        // compaction flags (config.ts:579-584)
        if flag::open_code_disable_autocompact() {
            apply_compaction_flag(&mut result, "auto", false);
        }
        if flag::open_code_disable_prune() {
            apply_compaction_flag(&mut result, "prune", false);
        }

        Ok(ConfigState {
            config: result,
            directories,
        })
    }

    /// Ported from: config.ts:606-608 (get) — cache tunggal per handle
    /// (InstanceState multi-directory ditunda; lihat DEVIATIONS.md).
    pub fn get(&self, ctx: &InstanceContext) -> Result<Value, ConfigLoadError> {
        let mut cache = self.instance_cache.lock().unwrap();
        if cache.is_none() {
            *cache = Some(self.load_instance_state(ctx)?);
        }
        Ok(cache.as_ref().unwrap().config.clone())
    }

    /// Ported from: config.ts:610-612 (directories)
    pub fn directories(&self, ctx: &InstanceContext) -> Result<Vec<PathBuf>, ConfigLoadError> {
        let mut cache = self.instance_cache.lock().unwrap();
        if cache.is_none() {
            *cache = Some(self.load_instance_state(ctx)?);
        }
        Ok(cache.as_ref().unwrap().directories.clone())
    }

    /// Hasil typed Info (padanan `Info` TS pada boundary API).
    pub fn get_info(&self, ctx: &InstanceContext) -> Result<Info, ConfigLoadError> {
        let value = self.get(ctx)?;
        parse::schema_decode::<Info>(value, "").map_err(ConfigLoadError::from)
    }
}

fn ensure_default_objects(result: &mut Value) {
    if let Value::Object(map) = result {
        map.entry("agent")
            .or_insert_with(|| Value::Object(Map::new()));
        map.entry("mode")
            .or_insert_with(|| Value::Object(Map::new()));
        map.entry("plugin")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
}

/// Ported from: config.ts:536-543 — result.mode dipromosikan ke result.agent
/// dengan mode:"primary".
fn promote_mode_to_agent(result: &mut Value) {
    let mode_entries = result
        .get("mode")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if mode_entries.is_empty() {
        return;
    }
    let mut agent = result
        .get("agent")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (name, mode_value) in mode_entries {
        if let Value::Object(mut mode_obj) = mode_value {
            mode_obj.insert("mode".to_string(), Value::String("primary".to_string()));
            agent.insert(name, Value::Object(mode_obj));
        }
    }
    if let Value::Object(map) = result {
        map.insert("agent".to_string(), Value::Object(agent));
    }
}

fn apply_compaction_flag(result: &mut Value, key: &str, value: bool) {
    // {...result.compaction, [key]: value}
    let existing = result
        .get("compaction")
        .cloned()
        .unwrap_or_else(empty_config);
    let mut compaction = match existing {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    compaction.insert(key.to_string(), Value::Bool(value));
    if let Value::Object(map) = result {
        map.insert("compaction".to_string(), Value::Object(compaction));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_deep_overrides_scalars_and_recurses_objects_only() {
        let target = serde_json::json!({"a": {"x": 1, "y": 2}, "list": [1,2], "keep": true});
        let source = serde_json::json!({"a": {"y": 9, "z": 3}, "list": [9]});
        let out = merge_deep(&target, &source);
        assert_eq!(
            out,
            serde_json::json!({"a": {"x": 1, "y": 9, "z": 3}, "list": [9], "keep": true})
        );
    }

    #[test]
    fn concat_arrays_dedupes_instructions_first_seen_order() {
        let target = serde_json::json!({"instructions": ["a.md","b.md"]});
        let source = serde_json::json!({"instructions": ["b.md","c.md"]});
        let out = merge_deep_concat_arrays(&target, &source);
        assert_eq!(
            out["instructions"],
            serde_json::json!(["a.md", "b.md", "c.md"])
        );
    }

    #[test]
    fn normalize_loaded_config_strips_legacy_keys_only_when_present() {
        let kept = normalize_loaded_config(serde_json::json!({"model": "m"}));
        assert_eq!(kept, serde_json::json!({"model": "m"}));

        let stripped = normalize_loaded_config(serde_json::json!({"theme": "dark", "model": "m"}));
        assert_eq!(stripped, serde_json::json!({"model": "m"}));
    }

    #[test]
    fn insert_schema_line_matches_regex_replacement() {
        let out = insert_schema_line("{\n  \"model\": \"m\"\n}");
        assert_eq!(
            out,
            "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"m\"\n}"
        );
        // tidak match → teks utuh
        assert_eq!(
            insert_schema_line("// leading comment\n{}"),
            "// leading comment\n{}"
        );
    }
}
