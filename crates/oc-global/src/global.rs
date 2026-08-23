//! Ported from: packages/core/src/global.ts
//! (dan dependency npm `xdg-basedir@5.1.0` yang dipakainya — semantik `||`
//! direplikasi persis: env var string kosong dianggap tidak ada.)

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::flag;

/// Ported from: packages/core/src/global.ts:10 (app, private const)
pub const APP: &str = "opencode";

fn os_homedir() -> Option<PathBuf> {
    // os.homedir(): USERPROFILE di Windows, $HOME di unix.
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

// --- xdg-basedir@5.1.0 (node_modules/xdg-basedir/index.js) ---

/// Ported from: xdg-basedir index.js (export const xdgData)
fn xdg_data_home() -> Option<PathBuf> {
    non_empty_env("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| os_homedir().map(|home| home.join(".local").join("share")))
}

/// Ported from: xdg-basedir index.js (export const xdgConfig)
fn xdg_config_home() -> Option<PathBuf> {
    non_empty_env("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| os_homedir().map(|home| home.join(".config")))
}

/// Ported from: xdg-basedir index.js (export const xdgState)
fn xdg_state_home() -> Option<PathBuf> {
    non_empty_env("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| os_homedir().map(|home| home.join(".local").join("state")))
}

/// Ported from: xdg-basedir index.js (export const xdgCache)
fn xdg_cache_home() -> Option<PathBuf> {
    non_empty_env("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| os_homedir().map(|home| home.join(".cache")))
}

fn non_empty_env(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

/// Ported from: packages/core/src/global.ts:17-29 (paths; `home` getter → method)
#[derive(Debug, Clone)]
pub struct Paths {
    pub data: PathBuf,
    pub bin: PathBuf,
    pub log: PathBuf,
    pub repos: PathBuf,
    pub cache: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub tmp: PathBuf,
}

impl Paths {
    /// Ported from: packages/core/src/global.ts:18-20 (getter home — evaluasi per akses)
    pub fn home(&self) -> PathBuf {
        match std::env::var_os("OPENCODE_TEST_HOME") {
            Some(value) => PathBuf::from(value),
            None => os_homedir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }
}

fn compute_paths() -> Paths {
    let app = APP;
    let join = |base: Option<PathBuf>| -> PathBuf {
        base.unwrap_or_else(|| panic!("xdg-basedir unresolved: os.homedir() is empty"))
    };
    let data = join(xdg_data_home()).join(app);
    let cache = join(xdg_cache_home()).join(app);
    let config = join(xdg_config_home()).join(app);
    let state = join(xdg_state_home()).join(app);
    let tmp = std::env::temp_dir().join(app);
    Paths {
        bin: cache.join("bin"),
        log: data.join("log"),
        repos: data.join("repos"),
        data,
        cache,
        config,
        state,
        tmp,
    }
}

static PATHS: RwLock<Option<Arc<Paths>>> = RwLock::new(None);

fn init_paths() -> Arc<Paths> {
    let paths = Arc::new(compute_paths());
    create_dirs(&paths);
    paths
}

/// Ported from: packages/core/src/global.ts:35-43 (mkdir saat import modul).
/// Di Rust efek samping import tidak ada; dijalankan sekali saat akses pertama
/// `path()` (atau eksplisit via fungsi ini).
pub fn ensure_dirs() {
    let _ = path();
}

fn create_dirs(paths: &Paths) {
    for dir in [
        &paths.data,
        &paths.config,
        &paths.state,
        &paths.tmp,
        &paths.log,
        &paths.bin,
        &paths.repos,
    ] {
        let _: std::io::Result<()> = fs_mkdir_all(dir);
    }
}

fn fs_mkdir_all(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// Ported from: packages/core/src/global.ts:31 (export const Path)
pub fn path() -> Arc<Paths> {
    {
        let guard = PATHS.read().unwrap();
        if let Some(paths) = guard.as_ref() {
            return paths.clone();
        }
    }
    let mut guard = PATHS.write().unwrap();
    let paths = guard.get_or_insert_with(init_paths);
    paths.clone()
}

/// Test infra saja (tidak ada di TS — module JS tidak bisa di-reload dengan env baru).
pub fn reset_for_tests() {
    *PATHS.write().unwrap() = None;
}

fn statics_snapshot(paths: &Paths) -> Interface {
    Interface {
        home: paths.home(),
        data: paths.data.clone(),
        cache: paths.cache.clone(),
        config: paths.config.clone(),
        state: paths.state.clone(),
        tmp: paths.tmp.clone(),
        bin: paths.bin.clone(),
        log: paths.log.clone(),
        repos: paths.repos.clone(),
    }
}

/// Ported from: packages/core/src/global.ts:47-57 (interface Interface)
#[derive(Debug, Clone)]
pub struct Interface {
    pub home: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub tmp: PathBuf,
    pub bin: PathBuf,
    pub log: PathBuf,
    pub repos: PathBuf,
}

/// Padanan `Partial<Interface>` untuk argumen `make`.
#[derive(Debug, Clone, Default)]
pub struct InterfaceOverride {
    pub home: Option<PathBuf>,
    pub data: Option<PathBuf>,
    pub cache: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub state: Option<PathBuf>,
    pub tmp: Option<PathBuf>,
    pub bin: Option<PathBuf>,
    pub log: Option<PathBuf>,
    pub repos: Option<PathBuf>,
}

/// Ported from: packages/core/src/global.ts:59-72 (make)
pub fn make(input: InterfaceOverride) -> Interface {
    let paths = path();
    let mut base = statics_snapshot(&paths);
    base.config = flag::open_code_config_dir()
        .map(PathBuf::from)
        .unwrap_or(base.config);
    if let Some(home) = input.home {
        base.home = home;
    }
    if let Some(data) = input.data {
        base.data = data;
    }
    if let Some(cache) = input.cache {
        base.cache = cache;
    }
    if let Some(config) = input.config {
        base.config = config;
    }
    if let Some(state) = input.state {
        base.state = state;
    }
    if let Some(tmp) = input.tmp {
        base.tmp = tmp;
    }
    if let Some(bin) = input.bin {
        base.bin = bin;
    }
    if let Some(log) = input.log {
        base.log = log;
    }
    if let Some(repos) = input.repos {
        base.repos = repos;
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    use std::sync::Mutex;

    fn set_env(key: &str, value: Option<&str>) {
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oc-global-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn xdg_defaults_and_custom_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("xdg");
        reset_for_tests();

        // default dari HOME/USERPROFILE
        set_env("HOME", Some(root.to_str().unwrap()));
        set_env("USERPROFILE", Some(root.to_str().unwrap()));
        set_env("XDG_DATA_HOME", None);
        set_env("XDG_CACHE_HOME", None);
        set_env("XDG_CONFIG_HOME", None);
        set_env("XDG_STATE_HOME", None);

        let paths = compute_paths();
        assert_eq!(paths.config, root.join(".config").join("opencode"));
        assert_eq!(paths.data, root.join(".local/share").join("opencode"));
        assert_eq!(paths.state, root.join(".local/state").join("opencode"));
        assert_eq!(paths.cache, root.join(".cache").join("opencode"));

        // custom XDG_CONFIG_HOME (kasus issue yang dirujuk sprint)
        let custom = root.join("custom-config");
        set_env("XDG_CONFIG_HOME", Some(custom.to_str().unwrap()));
        let paths = compute_paths();
        assert_eq!(paths.config, custom.join("opencode"));

        // env var kosong → fallback ke default (semantik `||` xdg-basedir v5)
        set_env("XDG_CONFIG_HOME", Some(""));
        let paths = compute_paths();
        assert_eq!(paths.config, root.join(".config").join("opencode"));

        reset_for_tests();
    }

    #[test]
    fn test_home_override_and_make_config_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("make");
        set_env(
            "OPENCODE_TEST_HOME",
            Some(root.join("fake-home").to_str().unwrap()),
        );
        set_env("OPENCODE_CONFIG_DIR", None);
        reset_for_tests();

        let default_iface = make(InterfaceOverride::default());
        assert_eq!(default_iface.home, root.join("fake-home"));

        let custom_dir = root.join("custom-dir");
        set_env("OPENCODE_CONFIG_DIR", Some(custom_dir.to_str().unwrap()));
        let overridden = make(InterfaceOverride::default());
        assert_eq!(overridden.config, custom_dir);
        // field lain tetap dari statik global
        assert_eq!(overridden.data, default_iface.data);

        set_env("OPENCODE_TEST_HOME", None);
        set_env("OPENCODE_CONFIG_DIR", None);
        reset_for_tests();
    }

    #[test]
    fn ensure_dirs_creates_directories() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_root("dirs");
        set_env("HOME", Some(root.to_str().unwrap()));
        set_env("USERPROFILE", Some(root.to_str().unwrap()));
        set_env("XDG_DATA_HOME", None);
        set_env("XDG_CACHE_HOME", None);
        set_env("XDG_CONFIG_HOME", None);
        set_env("XDG_STATE_HOME", None);
        set_env("OPENCODE_CONFIG_DIR", None);
        reset_for_tests();

        let paths = path();
        assert!(paths.data.exists(), "{:?} harus dibuat", paths.data);
        assert!(paths.config.exists());
        assert!(paths.state.exists());
        assert!(paths.tmp.exists());
        assert!(paths.log.exists());
        assert!(paths.bin.exists());
        assert!(paths.repos.exists());

        reset_for_tests();
    }

    #[test]
    fn tmp_uses_os_tmpdir() {
        let _guard = ENV_LOCK.lock().unwrap();
        reset_for_tests();
        let paths = compute_paths();
        assert_eq!(paths.tmp, std::env::temp_dir().join("opencode"));
        reset_for_tests();
    }
}
