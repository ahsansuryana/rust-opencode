//! Test oc-provider (Sprint 7a): sort/parse/default_model_ids, error messages,
//! ProviderAuth callback → oc-auth.

use std::collections::BTreeMap;

use oc_provider::auth::{Authorization, Method, Prompt, ProviderAuthService, SelectOption, When};
use oc_provider::{default_model_ids, parse_model, sort_models, Info, Model, Source};

fn model(id: &str) -> Model {
    Model {
        id: id.to_string(),
        provider_id: "test".into(),
        api: serde_json::json!({}),
        name: id.to_string(),
        family: None,
        capabilities: Default::default(),
        cost: Default::default(),
        limit: Default::default(),
        status: oc_provider::ModelStatus::Active,
        options: Default::default(),
        headers: Default::default(),
        release_date: String::new(),
    }
}

#[test]
fn parse_model_splits_on_first_slash() {
    let parsed = parse_model("anthropic/claude-sonnet-4");
    assert_eq!(parsed.provider_id, "anthropic");
    assert_eq!(parsed.model_id, "claude-sonnet-4");

    // modelID bisa mengandung slash lagi
    let parsed = parse_model("openrouter/deepseek/deepseek-r1");
    assert_eq!(parsed.provider_id, "openrouter");
    assert_eq!(parsed.model_id, "deepseek/deepseek-r1");
}

#[test]
fn sort_models_priority_latest_then_id_desc() {
    // priority desc: gemini-3-pro (idx 3) > claude-sonnet-4 (idx 1);
    // tanpa priority (-1) paling akhir. latest asc dulu di antara sama.
    let sorted = sort_models(&[
        "unknown-model".into(),
        "claude-sonnet-4-5".into(),
        "gemini-3-pro-preview".into(),
        "gpt-5-latest".into(),
    ]);
    // priorityIndex: gpt-5=0, claude=1, gemini=3, unknown=-1
    // desc → gemini(3), claude(1), gpt-5(0), unknown(-1)
    assert_eq!(sorted[0], "gemini-3-pro-preview");
    assert_eq!(sorted[1], "claude-sonnet-4-5");
    // gpt-5-latest vs ... hanya satu kandidat priority 0
    assert_eq!(sorted[2], "gpt-5-latest");
    assert_eq!(sorted[3], "unknown-model");
}

#[test]
fn default_model_ids_picks_sorted_first() {
    let mut providers = BTreeMap::new();
    providers.insert(
        "p1".to_string(),
        Info {
            id: "p1".into(),
            source: Source::Custom,
            name: "P1".into(),
            env: vec![],
            key: None,
            options: Default::default(),
            models: BTreeMap::from([
                ("z-model".to_string(), model("z-model")),
                ("gemini-3-pro-x".to_string(), model("gemini-3-pro-x")),
            ]),
        },
    );
    let defaults = default_model_ids(&providers);
    assert_eq!(defaults.get("p1").unwrap(), "gemini-3-pro-x");
}

#[test]
fn error_messages_match_ts_getters() {
    let e = oc_provider::error::ModelNotFoundError {
        provider_id: "anthropic".into(),
        model_id: "claude-9".into(),
        suggestions: Some(vec!["claude-3".into()]),
    };
    assert_eq!(
        e.to_string(),
        "Model not found: anthropic/claude-9. Did you mean: claude-3?"
    );
    let e = oc_provider::error::InitError {
        provider_id: "x".into(),
    };
    assert_eq!(e.to_string(), "Failed to initialize provider: x");
    let e = oc_provider::error::NoProvidersError;
    assert_eq!(e.to_string(), "No providers are available");
    let e = oc_provider::error::NoModelsError {
        provider_id: "y".into(),
    };
    assert_eq!(e.to_string(), "No models are available for provider: y");
}

// --- ProviderAuth ---

struct EnvGuard;
impl Drop for EnvGuard {
    fn drop(&mut self) {
        for k in [
            "HOME",
            "USERPROFILE",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "XDG_CONFIG_HOME",
            "XDG_STATE_HOME",
        ] {
            std::env::remove_var(k);
        }
        oc_global::reset_for_tests();
    }
}

#[test]
fn provider_auth_callback_writes_to_auth_json() {
    let root = std::env::temp_dir().join(format!("oc-provider-auth-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("HOME", root.to_str().unwrap());
    std::env::set_var("USERPROFILE", root.to_str().unwrap());
    oc_global::reset_for_tests();
    let _guard = EnvGuard;

    let service = ProviderAuthService::new(oc_auth::AuthService::new());

    // methods kosong tanpa plugin hooks
    assert!(service.methods().unwrap().is_empty());

    // authorize simpan pending
    let auth = Authorization {
        url: "https://auth.example.com/authorize".into(),
        method: "code".into(),
        instructions: "paste the code".into(),
    };
    let returned = service
        .authorize("acme", auth, &BTreeMap::new())
        .unwrap()
        .unwrap();
    assert_eq!(returned.url, "https://auth.example.com/authorize");

    // callback tanpa code saat method=code → OauthCodeMissing
    let err = service.callback("acme", None).unwrap_err();
    assert!(matches!(
        err,
        oc_provider::auth::ProviderAuthError::OauthCodeMissing { .. }
    ));

    // callback sukses → credential tertulis ke auth.json via oc-auth
    // (pending access kosong tanpa hook; uji jalur set_api_key yang dipakai
    //  cabang `"key" in result`)
    service.set_api_key("acme", "sk-test", None).unwrap();
    let stored = oc_auth::AuthService::new().get("acme").unwrap().unwrap();
    match stored {
        oc_auth::Info::Api(api) => assert_eq!(api.key, "sk-test"),
        other => panic!("variant salah: {other:?}"),
    }
}

#[test]
fn method_prompt_types_serialize_with_tag() {
    let method = Method {
        method_type: "oauth".into(),
        label: "Sign in".into(),
        prompts: Some(vec![Prompt::Select {
            key: "workspace".into(),
            message: "Pick workspace".into(),
            options: vec![SelectOption {
                label: "A".into(),
                value: "a".into(),
                hint: None,
            }],
            when: Some(When {
                key: "k".into(),
                op: "eq".into(),
                value: "v".into(),
            }),
        }]),
    };
    let json = serde_json::to_value(&method).unwrap();
    assert_eq!(json["type"], "oauth");
    assert_eq!(json["prompts"][0]["type"], "select");
}
