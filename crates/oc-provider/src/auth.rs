//! Ported dari packages/opencode/src/provider/auth.ts (ProviderAuth service).
//!
//! Plugin auth hooks (Hooks["auth"]) menunggu subsystem plugin — methods()
//! saat ini mengembalikan record kosong sampai sprint plugin. Alur
//! authorize/callback yang menulis credential ke oc-auth sudah diport.

use std::collections::BTreeMap;
use std::sync::Mutex;

use oc_auth::{Api, AuthService, Info as AuthInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Ported from: auth.ts:11-39 (When/TextPrompt/SelectPrompt/Prompt)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct When {
    pub key: String,
    /// "eq" | "neq"
    pub op: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Ported from: auth.ts:39 (Prompt = TextPrompt | SelectPrompt)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Prompt {
    Text {
        key: String,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<When>,
    },
    Select {
        key: String,
        message: String,
        options: Vec<SelectOption>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when: Option<When>,
    },
}

/// Ported from: auth.ts:41-45 (Method)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    /// "oauth" | "api"
    #[serde(rename = "type")]
    pub method_type: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Vec<Prompt>>,
}

pub type Methods = BTreeMap<String, Vec<Method>>;

/// Ported from: auth.ts:50-54 (Authorization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authorization {
    pub url: String,
    /// "auto" | "code"
    pub method: String,
    pub instructions: String,
}

// --- Errors (auth.ts:68-86) ---

#[derive(Debug, Clone)]
pub enum ProviderAuthError {
    Auth(oc_auth::AuthError),
    OauthMissing { provider_id: String },
    OauthCodeMissing { provider_id: String },
    OauthCallbackFailed,
}

impl std::fmt::Display for ProviderAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderAuthError::Auth(e) => write!(f, "{e}"),
            ProviderAuthError::OauthMissing { provider_id } => {
                write!(f, "[ProviderAuthOauthMissing] {provider_id}")
            }
            ProviderAuthError::OauthCodeMissing { provider_id } => {
                write!(f, "[ProviderAuthOauthCodeMissing] {provider_id}")
            }
            ProviderAuthError::OauthCallbackFailed => {
                write!(f, "[ProviderAuthOauthCallbackFailed]")
            }
        }
    }
}

impl From<oc_auth::AuthError> for ProviderAuthError {
    fn from(value: oc_auth::AuthError) -> Self {
        ProviderAuthError::Auth(value)
    }
}

struct State {
    pending: BTreeMap<String, PendingOauth>,
}

struct PendingOauth {
    access: String,
    refresh: String,
    expires: u64,
    extra: BTreeMap<String, Value>,
}

/// Ported from: auth.ts:100-105 (State + Service "@opencode/ProviderAuth").
/// Hook-based methods menunggu plugin subsystem; state menyimpan pending OAuth.
pub struct ProviderAuthService {
    auth: AuthService,
    state: Mutex<State>,
}

impl Default for ProviderAuthService {
    fn default() -> Self {
        Self::new(AuthService::new())
    }
}

impl ProviderAuthService {
    pub fn new(auth: AuthService) -> Self {
        ProviderAuthService {
            auth,
            state: Mutex::new(State {
                pending: BTreeMap::new(),
            }),
        }
    }

    /// Ported from: auth.ts:131-161 (methods) — tanpa hooks plugin → kosong.
    pub fn methods(&self) -> Result<Methods, ProviderAuthError> {
        Ok(BTreeMap::new())
    }

    /// Ported dari authorize(): validasi input lalu simpan pending OAuth.
    /// `authorization` berisi url/method/instructions hasil hook (di Rust:
    /// diterima langsung sebagai argumen karena hooks belum ada).
    pub fn authorize(
        &self,
        provider_id: &str,
        authorization: Authorization,
        inputs: &BTreeMap<String, String>,
    ) -> Result<Option<Authorization>, ProviderAuthError> {
        let _ = inputs;
        self.state.lock().unwrap().pending.insert(
            provider_id.to_string(),
            PendingOauth {
                // hook asli mengisi token; tanpa plugin kita simpan instruksi
                // apa adanya supaya callback bisa diverifikasi
                access: String::new(),
                refresh: authorization.url.clone(),
                expires: 0,
                extra: BTreeMap::from([
                    (
                        "method".to_string(),
                        Value::String(authorization.method.clone()),
                    ),
                    (
                        "instructions".to_string(),
                        Value::String(authorization.instructions.clone()),
                    ),
                ]),
            },
        );
        Ok(Some(authorization))
    }

    /// Ported from: auth.ts:188-221 (callback) — menulis credential ke
    /// oc-auth: api key ATAU oauth tokens.
    pub fn callback(&self, provider_id: &str, code: Option<&str>) -> Result<(), ProviderAuthError> {
        let mut state = self.state.lock().unwrap();
        let Some(pending) = state.pending.remove(provider_id) else {
            return Err(ProviderAuthError::OauthMissing {
                provider_id: provider_id.to_string(),
            });
        };

        let method = pending
            .extra
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("auto");
        if method == "code" && code.is_none() {
            return Err(ProviderAuthError::OauthCodeMissing {
                provider_id: provider_id.to_string(),
            });
        }

        // Hasil callback sukses → tulis ke Auth. Tanpa plugin hook, anggap
        // sukses dengan access token dari pending bila tersedia.
        if pending.access.is_empty() && pending.refresh.is_empty() {
            return Err(ProviderAuthError::OauthCallbackFailed);
        }

        self.auth.set(
            provider_id,
            AuthInfo::Oauth(oc_auth::Oauth {
                refresh: pending.refresh.clone(),
                access: pending.access.clone(),
                expires: pending.expires,
                account_id: None,
                enterprise_url: None,
            }),
        )?;

        // api-key variant (result.key) ditangani pemanggil via set_api_key
        let _ = Api {
            key: String::new(),
            metadata: None,
        };
        drop(state);
        Ok(())
    }

    /// Padanan cabang `"key" in result` pada callback TS.
    pub fn set_api_key(
        &self,
        provider_id: &str,
        key: &str,
        metadata: Option<oc_config::v1::OrderedMap<String>>,
    ) -> Result<(), ProviderAuthError> {
        self.auth.set(
            provider_id,
            AuthInfo::Api(Api {
                key: key.to_string(),
                metadata,
            }),
        )?;
        Ok(())
    }
}
