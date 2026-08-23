//! Ported from: packages/opencode/src/auth/index.ts

use std::path::PathBuf;

use oc_global::global;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use oc_config::v1::OrderedMap;

/// Ported from: packages/opencode/src/auth/index.ts:8 (OAUTH_DUMMY_KEY)
pub const OAUTH_DUMMY_KEY: &str = "opencode-oauth-dummy-key";

/// Ported from: packages/opencode/src/auth/index.ts:10 (`file` const modul)
pub fn auth_file() -> PathBuf {
    global::path().data.join("auth.json")
}

/// Ported from: packages/opencode/src/auth/index.ts:38-41 (AuthError)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Value>,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[AuthError] {}", self.message)
    }
}

impl std::error::Error for AuthError {}

fn auth_error(message: &str, cause: impl Into<Value>) -> AuthError {
    AuthError {
        message: message.to_string(),
        cause: Some(cause.into()),
    }
}

/// Ported from: packages/opencode/src/auth/index.ts:14-21 (Oauth / "OAuth")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Oauth {
    pub refresh: String,
    pub access: String,
    #[serde(deserialize_with = "non_negative")]
    pub expires: u64,
    #[serde(rename = "accountId", default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(
        rename = "enterpriseUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub enterprise_url: Option<String>,
}

fn non_negative<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i64::deserialize(deserializer)?;
    if value < 0 {
        return Err(serde::de::Error::custom("Expected non-negative integer"));
    }
    Ok(value as u64)
}

/// Ported from: packages/opencode/src/auth/index.ts:23-27 (Api / "ApiAuth")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Api {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<OrderedMap<String>>,
}

/// Ported from: packages/opencode/src/auth/index.ts:29-33 (WellKnown / "WellKnownAuth")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WellKnown {
    pub key: String,
    pub token: String,
}

/// Ported from: packages/opencode/src/auth/index.ts:35
/// (Info — union dengan discriminator "type")
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Info {
    #[serde(rename = "oauth")]
    Oauth(Oauth),
    #[serde(rename = "api")]
    Api(Api),
    #[serde(rename = "wellknown")]
    WellKnown(WellKnown),
}

fn strip_trailing_slashes(key: &str) -> String {
    // key.replace(/\/+$/, "")
    let trimmed = key.trim_end_matches('/');
    trimmed.to_string()
}

/// Ported dari FSUtil.readJson (core/fs-util.ts:102-108) + orElseSucceed({}):
/// file hilang ATAU JSON invalid → objek kosong.
fn read_json_value(path: &std::path::Path) -> Value {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            serde_json::from_str(&text).unwrap_or_else(|_| Value::Object(Default::default()))
        }
        Err(_) => Value::Object(Default::default()),
    }
}

/// Ported dari FSUtil.writeJson (core/fs-util.ts:110-114):
/// JSON.stringify(data, null, 2) lalu chmod 0600.
fn write_json_pretty_chmod(path: &std::path::Path, data: &Value) -> Result<(), AuthError> {
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| auth_error("Failed to write auth data", e.to_string()))?;
    std::fs::write(path, content)
        .map_err(|e| auth_error("Failed to write auth data", e.to_string()))?;
    set_mode_0600(path);
    Ok(())
}

fn set_mode_0600(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = match path.metadata() {
            Ok(metadata) => metadata.permissions(),
            Err(_) => return,
        };
        permissions.set_mode(0o600);
        let _ = std::fs::set_permissions(path, permissions);
    }
    #[cfg(not(unix))]
    {
        // Di Windows Node chmod 0600 tidak error; tidak ada padanan yang
        // perlu dilakukan — sengaja no-op.
        let _ = path;
    }
}

/// Ported from: packages/opencode/src/auth/index.ts:43-50
/// (Interface + Service "@opencode/Auth") — stateless service.
pub struct AuthService;

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthService {
    pub fn new() -> Self {
        AuthService
    }

    fn decode_map(data: Value) -> OrderedMap<Info> {
        let mut map = OrderedMap::new();
        if let Value::Object(entries) = data {
            for (key, value) in entries {
                if let Ok(info) = serde_json::from_value::<Info>(value) {
                    map.insert(key, info);
                }
            }
        }
        map
    }

    fn load_data(&self) -> OrderedMap<Info> {
        // process.env.OPENCODE_AUTH_CONTENT → JSON.parse; gagal parse diam-diam
        // jatuh ke file (try/catch kosong di TS).
        if let Ok(content) = std::env::var("OPENCODE_AUTH_CONTENT") {
            if let Ok(value) = serde_json::from_str::<Value>(&content) {
                return Self::decode_map(value);
            }
        }
        let data = read_json_value(&auth_file());
        Self::decode_map(data)
    }

    /// Ported from: index.ts:58-67 (all)
    pub fn all(&self) -> Result<OrderedMap<Info>, AuthError> {
        Ok(self.load_data())
    }

    /// Ported from: index.ts:69-71 (get)
    pub fn get(&self, provider_id: &str) -> Result<Option<Info>, AuthError> {
        Ok(self.all()?.get(provider_id).cloned())
    }

    /// Ported from: index.ts:73-81 (set)
    pub fn set(&self, key: &str, info: Info) -> Result<(), AuthError> {
        let norm = strip_trailing_slashes(key);
        let mut data = self.all()?;
        if norm != key {
            data.entries.retain(|(existing, _)| existing != key);
        }
        let slash_norm = format!("{norm}/");
        data.entries.retain(|(existing, _)| *existing != slash_norm);
        data.insert(norm, info);
        write_json_pretty_chmod(&auth_file(), &to_value(&data)?)
    }

    /// Ported from: index.ts:83-89 (remove)
    pub fn remove(&self, key: &str) -> Result<(), AuthError> {
        let norm = strip_trailing_slashes(key);
        let mut data = self.all()?;
        data.entries
            .retain(|(existing, _)| existing != key && *existing != norm);
        write_json_pretty_chmod(&auth_file(), &to_value(&data)?)
    }
}

fn to_value(map: &OrderedMap<Info>) -> Result<Value, AuthError> {
    serde_json::to_value(map).map_err(|e| auth_error("Failed to write auth data", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oc-auth-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn setup(root: &std::path::Path) {
        std::env::set_var("HOME", root.to_str().unwrap());
        std::env::set_var("USERPROFILE", root.to_str().unwrap());
        for key in [
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_CONFIG_HOME",
            "XDG_STATE_HOME",
        ] {
            std::env::remove_var(key);
        }
        oc_global::reset_for_tests();
        std::env::remove_var("OPENCODE_AUTH_CONTENT");
    }

    fn api_info(key: &str) -> Info {
        Info::Api(Api {
            key: key.into(),
            metadata: None,
        })
    }

    #[test]
    fn crud_and_key_normalization() {
        let _guard = env_lock();
        let root = temp_root("crud");
        setup(&root);

        let service = AuthService::new();

        // trailing slash dinormalisasi sebelum simpan
        service.set("anthropic/", api_info("sk-1")).unwrap();
        assert_eq!(service.get("anthropic").unwrap().unwrap(), api_info("sk-1"));

        // entri slash-suffix lama ikut terhapus saat set varian normal
        std::fs::write(auth_file(), r#"{"anthropic/":{"type":"api","key":"old"}}"#).unwrap();
        service.set("anthropic", api_info("sk-2")).unwrap();
        let all = service.all().unwrap();
        assert_eq!(all.len(), 1);
        assert!(all.get("anthropic").is_some());
        assert!(all.get("anthropic/").is_none());

        service.set("openai//", api_info("sk-oai")).unwrap();
        assert!(service.get("openai").unwrap().is_some());

        service.remove("anthropic").unwrap();
        assert!(service.get("anthropic").unwrap().is_none());
        service.remove("openai///").unwrap();
        assert!(service.all().unwrap().is_empty());
    }

    #[test]
    fn env_content_override_and_invalid_fallback() {
        let _guard = env_lock();
        let root = temp_root("env");
        setup(&root);

        std::env::set_var(
            "OPENCODE_AUTH_CONTENT",
            r#"{"x":{"type":"wellknown","key":"k","token":"t"},"broken":{"type":"api"}}"#,
        );
        let service = AuthService::new();
        let all = service.all().unwrap();
        // entri valid lolos, entri gagal schema dibuang diam-diam (filterMap)
        assert_eq!(all.len(), 1);
        assert!(matches!(all.get("x"), Some(Info::WellKnown(_))));

        // JSON invalid di env → fallback baca file
        std::fs::create_dir_all(global::path().data.clone()).unwrap();
        std::fs::write(auth_file(), r#"{"y":{"type":"api","key":"file-key"}}"#).unwrap();
        std::env::set_var("OPENCODE_AUTH_CONTENT", "{not json");
        let all = service.all().unwrap();
        assert_eq!(all.len(), 1);
        assert!(all.get("y").is_some());
    }

    #[test]
    fn round_trip_preserves_shape() {
        let _guard = env_lock();
        let root = temp_root("roundtrip");
        setup(&root);

        // fixture manual sesuai schema asli (campuran ketiga varian)
        let original = r#"{
  "anthropic": {
    "type": "oauth",
    "refresh": "r-1",
    "access": "a-1",
    "expires": 1700000000000,
    "accountId": "acc-1"
  },
  "openai": {
    "type": "api",
    "key": "sk-openai",
    "metadata": {"plan": "plus"}
  },
  "https://portal.example.com": {
    "type": "wellknown",
    "key": "TEST_TOKEN",
    "token": "tok-1"
  }
}"#;
        std::fs::create_dir_all(global::path().data.clone()).unwrap();
        std::fs::write(auth_file(), original).unwrap();

        let service = AuthService::new();
        let all = service.all().unwrap();
        assert_eq!(all.len(), 3);
        match all.get("anthropic") {
            Some(Info::Oauth(oauth)) => {
                assert_eq!(oauth.refresh, "r-1");
                assert_eq!(oauth.access, "a-1");
                assert_eq!(oauth.expires, 1700000000000);
                assert_eq!(oauth.account_id.as_deref(), Some("acc-1"));
                assert!(oauth.enterprise_url.is_none());
            }
            other => panic!("variant oauth salah: {other:?}"),
        }
        match all.get("openai") {
            Some(Info::Api(api)) => {
                assert_eq!(api.key, "sk-openai");
                assert_eq!(
                    api.metadata
                        .as_ref()
                        .unwrap()
                        .get("plan")
                        .map(String::as_str),
                    Some("plus")
                );
            }
            other => panic!("variant api salah: {other:?}"),
        }

        // tulis ulang → JSON semantically-equal dgn sumber (pretty, 2 spasi)
        service
            .set("anthropic", all.get("anthropic").unwrap().clone())
            .unwrap();
        let rewritten: Value =
            serde_json::from_str(&std::fs::read_to_string(auth_file()).unwrap()).unwrap();
        let source: Value = serde_json::from_str(original).unwrap();
        assert_eq!(rewritten, source);
    }

    #[cfg(unix)]
    #[test]
    fn file_permission_is_0600_on_unix() {
        let _guard = env_lock();
        let root = temp_root("perm");
        setup(&root);

        let service = AuthService::new();
        service.set("p", api_info("k")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(auth_file()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
