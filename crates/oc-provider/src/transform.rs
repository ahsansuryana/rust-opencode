//! Ported dari packages/opencode/src/provider/transform.ts bagian murni:
//! sampling params, effort computation, variants, options, schema sanitizers.

use serde_json::{json, Map, Value};

use crate::Model;

fn api_id(model: &Model) -> String {
    model.api["id"].as_str().unwrap_or_default().to_lowercase()
}

fn api_npm(model: &Model) -> String {
    model.api["npm"].as_str().unwrap_or_default().to_string()
}

pub const OUTPUT_TOKEN_MAX: usize = 32_000;

/// Ported from: transform.ts:521-526
const GEMINI_SAMPLING_DEFAULTS: &[&str] = &[
    "gemini-2.",
    "gemini-2-",
    "gemini-2_",
    "gemini-3-flash",
    "gemini-3-flash-",
    "gemini-3-pro",
    "gemini-3-pro-",
    "gemini-3.1",
    "gemini-3-1",
    "gemini-3_1",
    "gemini-3.5-flash",
];

fn gemini_sampling_match(id: &str) -> bool {
    // regex TS: /gemini-2[.-]5(?:[.-]|$)/ dsb — padankan awalan lalu batas [.-] atau akhir.
    // Untuk kesederhanaan dan fidelitas kasus umum: cek prefix + char berikutnya.
    for pat in GEMINI_SAMPLING_DEFAULTS {
        if let Some(rest) = id.strip_prefix(pat.trim_end_matches(['.', '-', '_'])) {
            if rest.is_empty() || rest.starts_with(['.', '-', '_']) {
                return true;
            }
        }
        // "gemini-2[.-]5" perlu perlakuan khusus: gemini-2.5 / gemini-2-5
        if *pat == "gemini-2." {
            if let Some(rest) = id.strip_prefix("gemini-2") {
                if rest.starts_with('.') || rest.starts_with('-') {
                    let after = &rest[1..];
                    let Some(tail) = after.strip_prefix('5') else {
                        continue;
                    };
                    if tail.is_empty() || tail.starts_with(['.', '-', '_']) {
                        return true;
                    }
                }
            }
            continue;
        }
    }
    false
}

/// Ported from: transform.ts:528-545 (temperature)
pub fn temperature(model: &Model) -> Option<f64> {
    let id = api_id(model);
    if id.contains("north-mini-code") {
        return Some(1.0);
    }
    if id.contains("claude") {
        return None;
    }
    if id.contains("gemini") {
        return if gemini_sampling_match(&id) {
            Some(1.0)
        } else {
            None
        };
    }
    if id.contains("glm-4.6") || id.contains("glm-4.7") || id.contains("minimax-m2") {
        return Some(1.0);
    }
    if id.contains("kimi-k2") {
        // kimi-k2-thinking & kimi-k2.5 && kimi-k2p5 && kimi-k2-5
        if ["thinking", "k2.", "k2p", "k2-5"]
            .iter()
            .any(|s| id.contains(s))
        {
            return Some(1.0);
        }
        return Some(0.6);
    }
    None
}

/// Ported from: transform.ts:547-561 (topP)
pub fn top_p(model: &Model) -> Option<f64> {
    let id = api_id(model);
    if id.contains("gemini") {
        return if gemini_sampling_match(&id) {
            Some(0.95)
        } else {
            None
        };
    }
    if ["minimax-m2", "kimi-k2.5", "kimi-k2p5", "kimi-k2-5"]
        .iter()
        .any(|s| id.contains(s))
    {
        return Some(0.95);
    }
    if ["deepseek-v4-flash-0731", "deepseek-v4-flash:0731"]
        .iter()
        .any(|n| id.contains(n))
        || (id.contains("deepseek-v4-flash")
            && (model.provider_id == "deepseek" || model.provider_id.starts_with("opencode")))
    {
        return Some(0.95);
    }
    None
}

/// Ported from: transform.ts:563-572 (topK)
pub fn top_k(model: &Model) -> Option<u32> {
    let id = api_id(model);
    if id.contains("minimax-m2") {
        if ["m2.", "m25", "m21"].iter().any(|s| id.contains(s)) {
            return Some(40);
        }
        return Some(20);
    }
    if id.contains("gemini") && gemini_sampling_match(&id) {
        return Some(64);
    }
    None
}

/// Ported from: transform.ts:1418-1420 (maxOutputTokens)
pub fn max_output_tokens(model: &Model, output_token_max: usize) -> usize {
    // Math.min(limit, max) || max — bila limit 0 → pakai max
    let value = (model.limit.output as usize).min(output_token_max);
    if value == 0 {
        output_token_max
    } else {
        value
    }
}

// --- OpenAI reasoning effort tiers ---

const WIDELY_SUPPORTED_EFFORTS: &[&str] = &["low", "medium", "high"];
const OPENAI_EFFORTS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];
const OPENAI_GPT5_1_EFFORTS: &[&str] = &["none", "low", "medium", "high"];
const OPENAI_GPT5_PRO_EFFORTS: &[&str] = &["high"];
const OPENAI_NONE_RELEASE: &str = "2025-11-13";
const OPENAI_XHIGH_RELEASE: &str = "2025-12-04";

/// /(?:^|\/)gpt-5(?:[.-]|$)/ — anchored ke awal atau "/"
fn gpt5_family(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower == "gpt-5"
        || lower.starts_with("gpt-5.")
        || lower.starts_with("gpt-5-")
        || lower.starts_with("/gpt-5")
            && (lower.len() == "/gpt-5".len()
                || lower["/gpt-5".len()..].starts_with('.')
                || lower["/gpt-5".len()..].starts_with('-'))
}

/// /(?:^|\/)gpt-5[.-](\d+)(?:[.-]|$)/ → versi mayor gpt-5.x
fn gpt5_version(api_id: &str) -> Option<u32> {
    let lower = api_id.to_lowercase();
    let start = if lower.starts_with("gpt-5.") || lower.starts_with("gpt-5-") {
        6usize
    } else {
        let pos = lower.find("/gpt-5.").or_else(|| lower.find("/gpt-5-"))?;
        pos + 7
    };
    let digits: String = lower[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn gpt5_pro(id: &str) -> bool {
    let lower = id.to_lowercase();
    let starts = |prefix: &str| lower.starts_with(prefix);
    let contains_after_slash = |needle: &str| {
        lower
            .find(needle)
            .map(|p| p == 0 || lower.as_bytes()[p - 1] == b'/')
            .unwrap_or(false)
    };
    starts("gpt-5.pro")
        || starts("gpt-5-pro")
        || starts("gpt-5pro")
        || contains_after_slash("gpt-5.pro")
        || contains_after_slash("gpt-5-pro")
}

/// Ported dari versionedGpt5ReasoningEfforts + konstanta tier TS.
fn openai_gpt5_1_efforts() -> Vec<&'static str> {
    OPENAI_GPT5_1_EFFORTS.to_vec()
}
fn openai_gpt5_2_plus_efforts() -> Vec<&'static str> {
    OPENAI_GPT5_1_EFFORTS
        .iter()
        .copied()
        .chain(std::iter::once("xhigh"))
        .collect()
}
fn openai_gpt5_codex_xhigh() -> Vec<&'static str> {
    WIDELY_SUPPORTED_EFFORTS
        .iter()
        .copied()
        .chain(std::iter::once("xhigh"))
        .collect()
}
fn openai_gpt5_codex_3_plus() -> Vec<&'static str> {
    std::iter::once("none")
        .chain(openai_gpt5_codex_xhigh())
        .collect()
}

fn versioned_gpt5_reasoning_efforts(api_id: &str) -> Option<Vec<&'static str>> {
    // GPT5_VERSIONED_PRO_RE: gpt-5.<digit>.pro / gpt-5-<digit>-pro
    let lower = api_id.to_lowercase();
    let version = gpt5_version(api_id);
    let versioned_pro = (version.is_some()) && (lower.contains(".pro") || lower.contains("-pro"));
    if versioned_pro {
        return Some(vec!["none", "low", "medium", "high", "xhigh"]);
    }
    match version {
        None => None,
        Some(1) => Some(openai_gpt5_1_efforts()),
        Some(_) => Some(openai_gpt5_2_plus_efforts()),
    }
}

fn gpt5_codex_reasoning_efforts(api_id: &str) -> Option<Vec<&'static str>> {
    if !gpt5_family(api_id) || !api_id.to_lowercase().contains("codex") {
        return None;
    }
    let version = gpt5_version(api_id);
    if let Some(v) = version {
        if v >= 3 {
            return Some(openai_gpt5_codex_3_plus());
        }
    }
    if api_id.to_lowercase().contains("codex-max") || version.map(|v| v >= 2).unwrap_or(false) {
        return Some(openai_gpt5_codex_xhigh());
    }
    Some(WIDELY_SUPPORTED_EFFORTS.to_vec())
}

fn gpt5_chat_reasoning_efforts(api_id: &str) -> Option<Vec<&'static str>> {
    if !gpt5_family(api_id) || !api_id.to_lowercase().contains("-chat") {
        return None;
    }
    if gpt5_version(api_id).is_none() {
        return Some(Vec::new());
    }
    Some(vec!["medium"])
}

/// Ported from: transform.ts:628-645 (openaiReasoningEfforts)
pub fn openai_reasoning_efforts(api_id: &str, release_date: &str) -> Vec<String> {
    let id = api_id.to_lowercase();
    if id.contains("deep-research") {
        return vec!["medium".to_string()];
    }
    if let Some(efforts) = gpt5_chat_reasoning_efforts(&id) {
        return efforts.into_iter().map(String::from).collect();
    }
    if gpt5_pro(&id) {
        return OPENAI_GPT5_PRO_EFFORTS
            .iter()
            .map(|s| s.to_string())
            .collect();
    }
    if let Some(efforts) = gpt5_codex_reasoning_efforts(&id) {
        return efforts.into_iter().map(String::from).collect();
    }
    if let Some(efforts) = versioned_gpt5_reasoning_efforts(&id) {
        return efforts.into_iter().map(String::from).collect();
    }
    let mut efforts: Vec<&str> = WIDELY_SUPPORTED_EFFORTS.to_vec();
    if gpt5_family(&id) {
        efforts.insert(0, "minimal");
    }
    if release_date >= OPENAI_NONE_RELEASE {
        efforts.insert(0, "none");
    }
    if release_date >= OPENAI_XHIGH_RELEASE {
        efforts.push("xhigh");
    }
    efforts.into_iter().map(String::from).collect()
}

/// Ported from: transform.ts:647-653 (openaiCompatibleReasoningEfforts)
pub fn openai_compatible_reasoning_efforts(id: &str) -> Vec<&'static str> {
    let api_id_lower = id.to_lowercase();
    if let Some(efforts) = gpt5_chat_reasoning_efforts(&api_id_lower) {
        return efforts;
    }
    if gpt5_pro(&api_id_lower) {
        return OPENAI_GPT5_PRO_EFFORTS.to_vec();
    }
    gpt5_codex_reasoning_efforts(&api_id_lower)
        .or_else(|| versioned_gpt5_reasoning_efforts(&api_id_lower))
        .unwrap_or_else(|| OPENAI_EFFORTS.to_vec())
}

// --- Anthropic adaptive thinking ---

/// Ported from: transform.ts:655-664 (anthropicUsesModernAdaptiveThinking)
pub fn anthropic_uses_modern_adaptive_thinking(api_id: &str) -> bool {
    if !api_id.to_lowercase().contains("claude-") {
        return false;
    }
    // claude-(family-)?MAJOR[.MINOR] dengan MINOR ≤2 digit
    let lower = api_id.to_lowercase();
    let Some(idx) = lower.find("claude-") else {
        return false;
    };
    let rest = &lower[idx + "claude-".len()..];
    let mut parts = rest.split(['.', '-', '@', '_', ':']);
    let family_or_major = parts.next().unwrap_or("");
    let (major, minor) = match family_or_major.parse::<u32>() {
        Ok(major) => {
            let minor = parts
                .next()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            (major, minor)
        }
        Err(_) => {
            // family-first: kata berikutnya adalah major; minor opsional ≤2 digit
            let Some(major) = parts.next().and_then(|s| s.parse::<u32>().ok()) else {
                return true;
            };
            let minor = parts
                .next()
                .and_then(|s| {
                    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if !digits.is_empty() && digits.len() <= 2 {
                        digits.parse::<u32>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            (major, minor)
        }
    };
    major > 4 || (major == 4 && minor >= 7)
}

/// Ported from: transform.ts:666-668 (anthropicOpus45)
pub fn anthropic_opus45(api_id: &str) -> bool {
    ["opus-4-5", "opus-4.5"].iter().any(|v| api_id.contains(v))
}

/// Ported from: transform.ts:670-682 (anthropicAdaptiveEfforts)
pub fn anthropic_adaptive_efforts(api_id: &str) -> Option<Vec<&'static str>> {
    if anthropic_uses_modern_adaptive_thinking(api_id) {
        return Some(vec!["low", "medium", "high", "xhigh", "max"]);
    }
    let matches = [
        "opus-4-6",
        "opus-4.6",
        "4-6-opus",
        "4.6-opus",
        "sonnet-4-6",
        "sonnet-4.6",
        "4-6-sonnet",
        "4.6-sonnet",
    ]
    .iter()
    .any(|v| api_id.contains(v));
    if matches {
        return Some(vec!["low", "medium", "high", "max"]);
    }
    None
}

/// Ported from: transform.ts:684-686 (anthropicOmitsThinking)
pub fn anthropic_omits_thinking(api_id: &str) -> bool {
    anthropic_uses_modern_adaptive_thinking(api_id)
}

// --- Google thinking ---

/// Ported from: transform.ts:688-695 (googleThinkingLevelEfforts)
pub fn google_thinking_level_efforts(api_id: &str) -> Vec<&'static str> {
    let id = api_id.to_lowercase();
    if !id.contains("gemini-3") {
        return vec!["low", "high"];
    }
    if id.contains("flash-image") {
        return vec!["minimal", "high"];
    }
    if id.contains("pro-image") {
        return vec!["high"];
    }
    if id.contains("flash") {
        return vec!["minimal", "low", "medium", "high"];
    }
    vec!["low", "medium", "high"]
}

/// Ported from: transform.ts:697-701 (googleThinkingBudgetMax)
pub fn google_thinking_budget_max(api_id: &str) -> u32 {
    let id = api_id.to_lowercase();
    if id.contains("2.5") && id.contains("pro") && !id.contains("flash") {
        return 32_768;
    }
    24_576
}

/// Ported from: transform.ts:709-725 (googleThinkingVariants)
pub fn google_thinking_variants(model: &Model) -> Map<String, Value> {
    let id = api_id(model);
    let mut out = Map::new();
    if id.contains("2.5") {
        out.insert(
            "high".to_string(),
            json!({ "thinkingConfig": { "includeThoughts": true, "thinkingBudget": 16000 } }),
        );
        out.insert(
            "max".to_string(),
            json!({ "thinkingConfig": { "includeThoughts": true, "thinkingBudget": google_thinking_budget_max(&id) } }),
        );
        return out;
    }
    for effort in google_thinking_level_efforts(&id) {
        out.insert(
            effort.to_string(),
            json!({ "thinkingConfig": { "includeThoughts": true, "thinkingLevel": effort } }),
        );
    }
    out
}

// --- schema sanitizers (transform.ts:1429-1652) ---

/// Ported from: transform.ts:1429-1510 (sanitizeOpenAISchema)
pub fn sanitize_openai_schema(value: &Value) -> Value {
    const TYPES: [&str; 7] = [
        "string", "number", "boolean", "integer", "object", "array", "null",
    ];
    const COMPOSITION_KEYS: [&str; 3] = ["anyOf", "oneOf", "allOf"];

    if value.is_boolean() {
        return json!({ "type": "string" });
    }
    if let Some(array) = value.as_array() {
        return Value::Array(array.iter().map(sanitize_openai_schema).collect());
    }
    let object = match value.as_object() {
        Some(map) => map,
        None => return value.clone(),
    };

    let mut result = Map::new();

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        result.insert("$ref".into(), json!(reference));
    }
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        result.insert("description".into(), json!(description));
    }
    if let Some(constant) = object.get("const") {
        result.insert("enum".into(), json!([constant]));
    } else if let Some(enum_values) = object.get("enum").and_then(Value::as_array) {
        result.insert("enum".into(), json!(enum_values));
    }

    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        result.insert(
            "properties".into(),
            Value::Object(
                properties
                    .iter()
                    .map(|(k, v)| (k.clone(), sanitize_openai_schema(v)))
                    .collect(),
            ),
        );
    }

    if let Some(required) = object.get("required").and_then(Value::as_array) {
        result.insert(
            "required".into(),
            json!(required
                .iter()
                .filter(|item| item.is_string())
                .cloned()
                .collect::<Vec<_>>()),
        );
    }

    if let Some(items) = object.get("items") {
        result.insert("items".into(), sanitize_openai_schema(items));
    }

    if let Some(additional) = object.get("additionalProperties") {
        result.insert(
            "additionalProperties".into(),
            match additional {
                Value::Bool(flag) => json!(flag),
                other => sanitize_openai_schema(other),
            },
        );
    }

    for key in COMPOSITION_KEYS {
        if let Some(array) = object.get(key).and_then(Value::as_array) {
            result.insert(
                key.to_string(),
                Value::Array(array.iter().map(sanitize_openai_schema).collect()),
            );
        }
    }

    for key in ["$defs", "definitions"] {
        if let Some(defs) = object.get(key).and_then(Value::as_object) {
            result.insert(
                key.to_string(),
                Value::Object(
                    defs.iter()
                        .map(|(k, v)| (k.clone(), sanitize_openai_schema(v)))
                        .collect(),
                ),
            );
        }
    }

    let schema_types: Vec<String> = match object.get("type") {
        Some(Value::String(single)) if TYPES.contains(&single.as_str()) => vec![single.clone()],
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_str)
            .filter(|item| TYPES.contains(item))
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };

    if schema_types.is_empty()
        && (result.contains_key("$ref")
            || COMPOSITION_KEYS.iter().any(|key| result.contains_key(*key)))
    {
        return Value::Object(result);
    }

    let inferred_types: Vec<String> = if !schema_types.is_empty() {
        schema_types
    } else if ["properties", "required", "additionalProperties"]
        .iter()
        .any(|key| object.contains_key(*key))
    {
        vec!["object".to_string()]
    } else if ["items", "prefixItems"]
        .iter()
        .any(|key| object.contains_key(*key))
    {
        vec!["array".to_string()]
    } else if result.contains_key("enum") || object.contains_key("format") {
        vec!["string".to_string()]
    } else if [
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
    {
        vec!["number".to_string()]
    } else {
        Vec::new()
    };

    if inferred_types.is_empty() {
        return json!({});
    }

    result.insert(
        "type".into(),
        if inferred_types.len() == 1 {
            json!(inferred_types[0])
        } else {
            json!(inferred_types)
        },
    );
    if inferred_types.iter().any(|t| t == "object") && !result.contains_key("properties") {
        result.insert("properties".into(), json!({}));
    }
    if inferred_types.iter().any(|t| t == "array") && !result.contains_key("items") {
        result.insert("items".into(), json!({ "type": "string" }));
    }
    Value::Object(result)
}

/// Ported from: transform.ts:1537-1546 (sanitizeMoonshot)
fn sanitize_moonshot(obj: &Value) -> Value {
    match obj {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => obj.clone(),
        Value::Array(items) => Value::Array(items.iter().map(sanitize_moonshot).collect()),
        Value::Object(map) => {
            if let Some(r#ref) = map.get("$ref").and_then(Value::as_str) {
                return json!({ "$ref": r#ref });
            }
            let result: Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), sanitize_moonshot(v)))
                .collect();
            let mut result = result;
            if let Some(items) = result.get_mut("items") {
                if let Value::Array(list) = items {
                    let first = list.first().cloned().unwrap_or(json!({}));
                    *items = first;
                }
            }
            Value::Object(result)
        }
    }
}

fn has_combiner(node: &Value) -> bool {
    node.as_object()
        .map(|m| {
            ["anyOf", "oneOf", "allOf"]
                .iter()
                .any(|k| m.get(*k).map(Value::is_array).unwrap_or(false))
        })
        .unwrap_or(false)
}

fn has_schema_intent(node: &Value) -> bool {
    let Some(map) = node.as_object() else {
        return false;
    };
    if has_combiner(node) {
        return true;
    }
    [
        "type",
        "properties",
        "items",
        "prefixItems",
        "enum",
        "const",
        "$ref",
        "additionalProperties",
        "patternProperties",
        "required",
        "not",
        "if",
        "then",
        "else",
    ]
    .iter()
    .any(|key| map.contains_key(*key))
}

/// Ported from: transform.ts:1581-1646 (sanitizeGemini)
fn sanitize_gemini(obj: &Value) -> Value {
    match obj {
        Value::Object(map) => {
            let mut result = Map::new();
            for (key, value) in map {
                if key == "enum" {
                    if let Some(values) = value.as_array() {
                        result.insert(
                            key.clone(),
                            Value::Array(values.iter().map(|v| json!(v.to_string())).collect()),
                        );
                        if result.get("type") == Some(&json!("integer"))
                            || result.get("type") == Some(&json!("number"))
                        {
                            result.insert("type".into(), json!("string"));
                        }
                        continue;
                    }
                }
                if value.is_object() {
                    result.insert(key.clone(), sanitize_gemini(value));
                } else {
                    result.insert(key.clone(), value.clone());
                }
            }

            // type array → anyOf + nullable
            if let Some(Value::Array(types)) = result.get("type").cloned().as_ref() {
                let has_null = types.iter().any(|t| t.as_str() == Some("null"));
                let non_null: Vec<Value> = types
                    .iter()
                    .filter(|t| t.as_str() != Some("null"))
                    .cloned()
                    .collect();
                if non_null.is_empty() {
                    result.insert("type".into(), json!("null"));
                } else {
                    result.remove("type");
                    result.insert(
                        "anyOf".into(),
                        Value::Array(
                            non_null
                                .into_iter()
                                .map(|entry| json!({ "type": entry }))
                                .collect(),
                        ),
                    );
                    if has_null {
                        result.insert("nullable".into(), json!(true));
                    }
                }
            }

            // required hanya field yang ada di properties
            if result.get("type").and_then(Value::as_str) == Some("object")
                && result.contains_key("properties")
            {
                if let Some(required) = result.get("required").and_then(Value::as_array).cloned() {
                    if let Some(properties) =
                        result.get("properties").and_then(Value::as_object).cloned()
                    {
                        result.insert(
                            "required".into(),
                            json!(required
                                .into_iter()
                                .filter(|field| properties
                                    .contains_key(field.as_str().unwrap_or_default()))
                                .collect::<Vec<_>>()),
                        );
                    }
                }
            }

            // array tanpa items bertipe → items string
            if result.get("type").and_then(Value::as_str) == Some("array")
                && !has_combiner(&Value::Object(result.clone()))
            {
                if !result.contains_key("items") {
                    result.insert("items".into(), json!({}));
                }
                if let Some(Value::Object(items)) = result.get("items") {
                    if !has_schema_intent(&Value::Object(items.clone())) {
                        let mut items = items.clone();
                        items.insert("type".into(), json!("string"));
                        result.insert("items".into(), Value::Object(items));
                    }
                }
            }

            // buang properties/required dari non-object
            let type_is_non_object = result
                .get("type")
                .and_then(Value::as_str)
                .map(|t| !t.is_empty() && t != "object")
                .unwrap_or(false);
            if type_is_non_object && !has_combiner(&Value::Object(result.clone())) {
                result.remove("properties");
                result.remove("required");
            }

            Value::Object(result)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_gemini).collect()),
        other => other.clone(),
    }
}

/// Ported from: transform.ts:1512-1652 (schema)
pub fn schema(model: &Model, schema_value: &Value) -> Value {
    let mut schema_value = schema_value.clone();
    let npm = api_npm(model);
    if npm == "@ai-sdk/openai" || npm == "@ai-sdk/azure" {
        schema_value = sanitize_openai_schema(&schema_value);
    }

    let id = api_id(model);
    if model.provider_id == "moonshotai" || id.contains("kimi") {
        let sanitized = sanitize_moonshot(&schema_value);
        if sanitized.is_object() {
            schema_value = sanitized;
        }
    }

    if model.provider_id == "google" || id.contains("gemini") {
        schema_value = sanitize_gemini(&schema_value);
    }

    schema_value
}
