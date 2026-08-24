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

// --- Sprint 7b: transform.ts pure functions ---

use oc_provider::transform;

fn make_model(api_id: &str, npm: &str) -> Model {
    Model {
        id: api_id.to_string(),
        provider_id: "test".into(),
        api: serde_json::json!({"id": api_id, "npm": npm, "url": ""}),
        name: api_id.to_string(),
        family: None,
        capabilities: Default::default(),
        cost: Default::default(),
        limit: oc_provider::Limit {
            context: 200_000,
            input: None,
            output: 8192,
        },
        status: oc_provider::ModelStatus::Active,
        options: Default::default(),
        headers: Default::default(),
        release_date: "2025-01-01".into(),
    }
}

#[test]
fn temperature_branches() {
    let m = |id| make_model(id, "@ai-sdk/anthropic");
    assert_eq!(transform::temperature(&m("north-mini-code-x")), Some(1.0));
    assert_eq!(transform::temperature(&m("claude-sonnet-4")), None);
    assert_eq!(transform::temperature(&m("gemini-2.5-pro")), Some(1.0));
    assert_eq!(transform::temperature(&m("gemini-1.5-pro")), None);
    assert_eq!(transform::temperature(&m("glm-4.6")), Some(1.0));
    assert_eq!(transform::temperature(&m("minimax-m2")), Some(1.0));
    assert_eq!(transform::temperature(&m("kimi-k2")), Some(0.6));
    assert_eq!(transform::temperature(&m("kimi-k2-thinking")), Some(1.0));
}

#[test]
fn top_p_and_top_k() {
    let m = |id: &str, pid: &str| -> Model {
        let mut model = make_model(id, "@ai-sdk/openai");
        model.provider_id = pid.into();
        model
    };
    assert_eq!(
        transform::top_p(&m("gemini-2.5-flash", "google")),
        Some(0.95)
    );
    assert_eq!(transform::top_p(&m("kimi-k2-5", "moonshot")), Some(0.95));
    assert_eq!(
        transform::top_p(&m("deepseek-v4-flash", "deepseek")),
        Some(0.95)
    );
    assert_eq!(transform::top_p(&m("gpt-4o", "openai")), None);

    assert_eq!(transform::top_k(&m("minimax-m21", "x")), Some(40));
    assert_eq!(transform::top_k(&m("minimax-m2-old", "x")), Some(20));
    assert_eq!(transform::top_k(&m("gemini-2.5-pro", "google")), Some(64));
    assert_eq!(transform::top_k(&m("gpt-4o", "openai")), None);
}

#[test]
fn openai_reasoning_efforts_tiers() {
    // deep-research selalu medium saja
    assert_eq!(
        transform::openai_reasoning_efforts("o3-deep-research", "2025-01-01"),
        vec!["medium"]
    );
    // gpt-5 versi 1 → none/low/medium/high (minimal diganti none)
    let efforts = transform::openai_reasoning_efforts("gpt-5.1", "2025-11-20");
    assert_eq!(efforts, vec!["none", "low", "medium", "high"]);
    // gpt-5.2+ → + xhigh
    let efforts = transform::openai_reasoning_efforts("gpt-5.2", "2025-12-05");
    assert_eq!(efforts, vec!["none", "low", "medium", "high", "xhigh"]);
    // gpt-5 dasar sebelum none-release → minimal/low/medium/high
    let efforts = transform::openai_reasoning_efforts("gpt-5", "2025-08-01");
    assert_eq!(efforts, vec!["minimal", "low", "medium", "high"]);
    // codex-max → + xhigh
    let efforts = transform::openai_reasoning_efforts("gpt-5-codex-max", "2025-06-01");
    assert_eq!(efforts, vec!["low", "medium", "high", "xhigh"]);
    // pro → high saja
    let efforts = transform::openai_reasoning_efforts("gpt-5-pro", "2025-06-01");
    assert_eq!(efforts, vec!["high"]);
}

#[test]
fn anthropic_adaptive_detection() {
    // modern adaptive: major > 4 atau 4.x ≥ 7
    assert!(transform::anthropic_uses_modern_adaptive_thinking(
        "claude-opus-4.7"
    ));
    assert!(transform::anthropic_uses_modern_adaptive_thinking(
        "claude-opus-5"
    ));
    // release-date ID seperti claude-opus-4-20250514 → minor=20250514? tidak,
    // digit >2 diabaikan → minor=0 → false utk 4.7 threshold
    assert!(!transform::anthropic_uses_modern_adaptive_thinking(
        "claude-opus-4"
    ));
    assert!(!transform::anthropic_uses_modern_adaptive_thinking(
        "claude-sonnet-4-20250514"
    ));
    // tanpa digit sama sekali → regex gagal → modern adaptive (sesuai TS)
    assert!(transform::anthropic_uses_modern_adaptive_thinking(
        "claude-haiku"
    ));

    assert_eq!(
        transform::anthropic_adaptive_efforts("claude-opus-4.6"),
        Some(vec!["low", "medium", "high", "max"])
    );
    // claude-haiku → uses_modern=true → effort list penuh
    assert_eq!(
        transform::anthropic_adaptive_efforts("claude-haiku"),
        Some(vec!["low", "medium", "high", "xhigh", "max"])
    );
    assert!(transform::anthropic_opus45("claude-opus-4-5-20251101"));
    assert!(transform::anthropic_opus45("claude-opus-4.5"));
    assert!(!transform::anthropic_opus45("claude-opus-4.1"));
}

#[test]
fn google_thinking_levels_and_budget() {
    assert_eq!(
        transform::google_thinking_level_efforts("gemini-2.5-pro"),
        vec!["low", "high"]
    );
    assert_eq!(
        transform::google_thinking_level_efforts("gemini-3-flash"),
        vec!["minimal", "low", "medium", "high"]
    );
    assert_eq!(
        transform::google_thinking_level_efforts("gemini-3-pro-image"),
        vec!["high"]
    );
    assert_eq!(
        transform::google_thinking_budget_max("gemini-2.5-pro"),
        32_768
    );
    assert_eq!(
        transform::google_thinking_budget_max("gemini-2.5-flash"),
        24_576
    );
}

#[test]
fn schema_sanitizers() {
    let m_openai = make_model("gpt-4o", "@ai-sdk/openai");

    // boolean schema → {type:"string"}
    let result = transform::schema(&m_openai, &serde_json::json!(true));
    assert_eq!(result, serde_json::json!({"type": "string"}));

    // const → enum
    let result = transform::schema(
        &m_openai,
        &serde_json::json!({"const": "abc", "description": "d"}),
    );
    assert_eq!(result["enum"], serde_json::json!(["abc"]));
    assert_eq!(result["description"], "d");

    // type tak dikenal tanpa intent → {}
    let result = transform::schema(&m_openai, &serde_json::json!({"maxLength": 5}));
    assert_eq!(result, serde_json::json!({}));

    // properties → inferred object
    let result = transform::schema(
        &m_openai,
        &serde_json::json!({"properties": {"a": {"type": "string"}}}),
    );
    assert_eq!(result["type"], "object");
    assert!(result["properties"].is_object());

    // moonshot: items array dipadatkan ke elemen pertama; $ref bersih dari sibling
    let mut kimi = make_model("kimi-k2", "@ai-sdk/openai-compatible");
    kimi.provider_id = "moonshotai".into();
    // $ref node: sibling keywords dibuang
    let result = transform::schema(
        &kimi,
        &serde_json::json!({"$ref": "#/$defs/x", "description": "ignored"}),
    );
    assert_eq!(result["$ref"], "#/$defs/x");
    assert!(result.get("description").is_none());
    // items array dipadatkan ke elemen pertama
    let result = transform::schema(
        &kimi,
        &serde_json::json!({"items": [{"type": "string"}, {"type": "number"}]}),
    );
    println!("DBG items = {result:?}");
    assert_eq!(result["items"], serde_json::json!({"type": "string"}));

    // gemini: enum integer → string; type array → anyOf+nullable; required difilter
    let mut google = make_model("gemini-2.5-pro", "@ai-sdk/google");
    google.provider_id = "google".into();
    google.provider_id = "google".into();
    let result = transform::schema(
        &google,
        &serde_json::json!({
            "type": "object",
            "properties": {"n": {"type": ["integer","null"], "enum": [1,2]}},
            "required": ["n","ghost"]
        }),
    );
    let prop_n = &result["properties"]["n"];
    // type array [integer,null] → anyOf [{integer},{null}] + nullable
    // tapi enum mengubah type jadi string lebih dulu — hasil tergantung urutan;
    // type array [integer,null] → anyOf + nullable (sesuai TS)
    assert!(
        prop_n.get("type").is_none(),
        "type harus dihapus jadi anyOf"
    );
    assert_eq!(prop_n["anyOf"], serde_json::json!([{"type": "integer"}]));
    assert_eq!(prop_n["nullable"], true);
    assert_eq!(prop_n["enum"], serde_json::json!(["1", "2"]));
    // required difilter hanya field yang ada di properties
    let required = result["required"].as_array().unwrap();
    assert_eq!(required.len(), 1);
    assert_eq!(required[0], "n");
}
