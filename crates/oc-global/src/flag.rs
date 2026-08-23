//! Ported from: packages/core/src/flag/flag.ts
//!
//! PORT SEBAGIAN: hanya flag yang dipakai oc-global/oc-config saat ini
//! (lihat NAMING_MAP.md). Flag lain ditambahkan sprint berikutnya.
//! Flag non-getter TS dievaluasi saat module load → di sini sekali via cache;
//! getter TS dievaluasi per akses → fn biasa per-call.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Ported from: packages/core/src/flag/flag.ts:3-6 (truthy)
pub fn truthy(key: &str) -> bool {
    match std::env::var(key) {
        Ok(value) => {
            let lower = value.to_lowercase();
            lower == "true" || lower == "1"
        }
        Err(_) => false,
    }
}

fn env_once(key: &'static str) -> Option<String> {
    let mutex = ENV_STRING_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = mutex.lock().unwrap();
    if let Some(cached) = map.get(key) {
        return cached.clone();
    }
    let value = std::env::var(key).ok();
    map.insert(key, value.clone());
    value
}

fn truthy_once(key: &'static str) -> bool {
    let mutex = ENV_BOOL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = mutex.lock().unwrap();
    if let Some(cached) = map.get(key) {
        return *cached;
    }
    let value = truthy(key);
    map.insert(key, value);
    value
}

/// Ported from: packages/core/src/flag/flag.ts:21 (OPENCODE_CONFIG, module-load eval)
pub fn open_code_config() -> Option<String> {
    env_once("OPENCODE_CONFIG")
}

/// Ported from: packages/core/src/flag/flag.ts:22 (OPENCODE_CONFIG_CONTENT, module-load eval)
pub fn open_code_config_content() -> Option<String> {
    env_once("OPENCODE_CONFIG_CONTENT")
}

/// Ported from: packages/core/src/flag/flag.ts:63-65 (getter OPENCODE_CONFIG_DIR)
pub fn open_code_config_dir() -> Option<String> {
    std::env::var("OPENCODE_CONFIG_DIR").ok()
}

/// Ported from: packages/core/src/flag/flag.ts:54-56 (getter OPENCODE_DISABLE_PROJECT_CONFIG)
pub fn open_code_disable_project_config() -> bool {
    truthy("OPENCODE_DISABLE_PROJECT_CONFIG")
}

/// Ported from: packages/core/src/flag/flag.ts:69-71 (getter OPENCODE_PERMISSION)
pub fn open_code_permission() -> Option<String> {
    std::env::var("OPENCODE_PERMISSION").ok()
}

/// Ported from: packages/core/src/flag/flag.ts:28 (OPENCODE_DISABLE_AUTOCOMPACT, module-load eval)
pub fn open_code_disable_autocompact() -> bool {
    truthy_once("OPENCODE_DISABLE_AUTOCOMPACT")
}

/// Ported from: packages/core/src/flag/flag.ts:25 (OPENCODE_DISABLE_PRUNE, module-load eval)
pub fn open_code_disable_prune() -> bool {
    truthy_once("OPENCODE_DISABLE_PRUNE")
}

/// Test infra saja (tidak ada di TS): mengosongkan cache evaluasi module-load
/// supaya test bisa menyetel ulang env var.
pub fn reset_for_tests() {
    ENV_STRING_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .clear();
    ENV_BOOL_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .clear();
}

static ENV_STRING_CACHE: OnceLock<Mutex<HashMap<&'static str, Option<String>>>> = OnceLock::new();
static ENV_BOOL_CACHE: OnceLock<Mutex<HashMap<&'static str, bool>>> = OnceLock::new();
