//! Ported dari packages/opencode/src/provider/transform.ts:727-1350
//! (variants — switch per-npm) dan 1157-1416 (options, smallOptions,
//! providerOptions). Fungsi murni berbasis Model + Value.

use serde_json::{json, Map, Value};

use crate::transform::{
    anthropic_adaptive_efforts, anthropic_omits_thinking, anthropic_opus45,
    google_thinking_budget_max, google_thinking_level_efforts, openai_compatible_reasoning_efforts,
    openai_reasoning_efforts,
};
use crate::Model;

fn api_id(model: &Model) -> String {
    model.api["id"].as_str().unwrap_or_default().to_lowercase()
}

fn api_npm(model: &Model) -> String {
    model.api["npm"].as_str().unwrap_or_default().to_string()
}

const INCLUDE_ENCRYPTED_REASONING: &str = "reasoning.encrypted_content";
const WIDELY_SUPPORTED_EFFORTS: &[&str] = &["low", "medium", "high"];

fn efforts_map(efforts: &[&str], body: Value) -> Map<String, Value> {
    let mut out = Map::new();
    for effort in efforts {
        out.insert(effort.to_string(), json!(body));
    }
    out
}

fn is_kimi_family(model: &Model) -> bool {
    let api_id_str = api_id(model);
    let ids = [model.provider_id.as_str(), api_id_str.as_str()];
    for id in ids {
        let value = id.to_lowercase();
        if value.contains("kimi") || value.contains("moonshot") {
            return true;
        }
    }
    let url = model.api["url"].as_str().unwrap_or("").to_lowercase();
    [
        "api.kimi.com",
        "api.moonshot.ai",
        "api.moonshot.cn",
        "api.moonshotai.cn",
    ]
    .iter()
    .any(|host| url.contains(host))
}

/// Ported from: transform.ts:727-1155 (variants)
pub fn variants(model: &Model) -> Map<String, Value> {
    if !model.capabilities.reasoning {
        return Map::new();
    }
    let id = model.id.to_lowercase();
    let npm = api_npm(model);
    let api_id_lower = api_id(model);

    let glm52 = ["glm-5.2", "glm-5-2", "glm-5p2"]
        .iter()
        .any(|name| id.contains(name) || api_id_lower.contains(name));

    if api_id_lower.contains("minimax-m3")
        && ["@ai-sdk/anthropic", "@ai-sdk/openai-compatible"].contains(&npm.as_str())
    {
        if ["nvidia", "lilac"].contains(&model.provider_id.as_str()) {
            return serde_json::from_value(json!({
                "none": { "chat_template_kwargs": { "thinking_mode": "disabled" } },
                "thinking": { "chat_template_kwargs": { "thinking_mode": "enabled" } }
            }))
            .unwrap_or_default();
        }
        return serde_json::from_value(json!({
            "none": { "thinking": { "type": "disabled" } },
            "thinking": { "thinking": { "type": "adaptive" } }
        }))
        .unwrap_or_default();
    }

    let adaptive_omitted = anthropic_omits_thinking(&api_id(model));
    let adaptive = anthropic_adaptive_efforts(&api_id(model));

    if glm52 && npm == "@openrouter/ai-sdk-provider" {
        return serde_json::from_value(json!({
            "high": { "reasoning": { "effort": "high" } },
            "xhigh": { "reasoning": { "effort": "xhigh" } }
        }))
        .unwrap_or_default();
    }
    if glm52 && npm == "@ai-sdk/openai-compatible" {
        return serde_json::from_value(json!({
            "high": { "reasoningEffort": "high" },
            "max": { "reasoningEffort": "max" }
        }))
        .unwrap_or_default();
    }
    if glm52 && npm == "@ai-sdk/anthropic" {
        return serde_json::from_value(json!({
            "high": { "effort": "high" },
            "max": { "effort": "max" }
        }))
        .unwrap_or_default();
    }
    // Kimi adaptive via Anthropic-compatible transports
    if is_kimi_family(model)
        && ["@ai-sdk/anthropic", "@ai-sdk/google-vertex/anthropic"].contains(&npm.as_str())
    {
        let mut out = Map::new();
        for effort in ["low", "medium", "high", "xhigh", "max"] {
            out.insert(
                effort.to_string(),
                json!({ "thinking": { "type": "adaptive", "display": "summarized" }, "effort": effort }),
            );
        }
        return out;
    }
    // skip variants untuk keluarga tanpa reasoning controls
    let skip = [
        "deepseek-chat",
        "deepseek-reasoner",
        "deepseek-r1",
        "deepseek-v3",
        "minimax",
    ]
    .iter()
    .any(|s| id.contains(s))
        || (id.contains("glm") && !glm52)
        || id.contains("kimi")
        || id.contains("k2p")
        || id.contains("qwen")
        || id.contains("big-pickle");
    if skip {
        return Map::new();
    }
    // grok-3-mini
    if id.contains("grok") && id.contains("grok-3-mini") {
        if npm == "@openrouter/ai-sdk-provider" {
            return serde_json::from_value(json!({
                "low": { "reasoning": { "effort": "low" } },
                "high": { "reasoning": { "effort": "high" } }
            }))
            .unwrap_or_default();
        }
        return serde_json::from_value(json!({
            "low": { "reasoningEffort": "low" },
            "high": { "reasoningEffort": "high" }
        }))
        .unwrap_or_default();
    }

    match npm.as_str() {
        "@openrouter/ai-sdk-provider" => {
            let efforts: Vec<&str> = if api_id(model).starts_with("openai/") || id.contains("gpt") {
                openai_compatible_reasoning_efforts(&api_id(model))
            } else {
                WIDELY_SUPPORTED_EFFORTS.to_vec()
            };
            efforts_map(&efforts, json!({ "reasoning": { "effort": "" } }))
        }
        "ai-gateway-provider" => {
            if api_id(model).starts_with("openai/") {
                let efforts = openai_reasoning_efforts(&api_id(model), &model.release_date);
                let mut out = Map::new();
                for e in efforts {
                    out.insert(e.clone(), json!({ "reasoningEffort": e }));
                }
                return out;
            }
            efforts_map(WIDELY_SUPPORTED_EFFORTS, json!({ "reasoningEffort": "" }))
        }
        "@ai-sdk/gateway" => {
            if api_id_lower.contains("anthropic") {
                if let Some(adaptive_efforts) = &adaptive {
                    let mut out = Map::new();
                    for effort in adaptive_efforts {
                        let mut thinking = json!({ "type": "adaptive" });
                        if adaptive_omitted {
                            thinking["display"] = json!("summarized");
                        }
                        out.insert(
                            effort.to_string(),
                            json!({ "thinking": thinking, "effort": effort }),
                        );
                    }
                    return out;
                }
                return serde_json::from_value(json!({
                    "high": { "thinking": { "type": "enabled", "budgetTokens": 16000 } },
                    "max": { "thinking": { "type": "enabled", "budgetTokens": 31999 } }
                }))
                .unwrap_or_default();
            }
            if api_id_lower.contains("google") {
                if api_id_lower.contains("2.5") {
                    return google_thinking_variants(model);
                }
                let mut out = Map::new();
                for effort in ["low", "high"] {
                    out.insert(
                        effort.to_string(),
                        json!({ "includeThoughts": true, "thinkingLevel": effort }),
                    );
                }
                return out;
            }
            efforts_map(
                &openai_compatible_reasoning_efforts(&api_id(model)),
                json!({ "reasoningEffort": "" }),
            )
        }
        "@ai-sdk/github-copilot" => {
            if model.id.contains("gemini") {
                return Map::new();
            }
            if model.id.contains("claude") {
                return efforts_map(WIDELY_SUPPORTED_EFFORTS, json!({ "reasoningEffort": "" }));
            }
            let copilot_efforts: Vec<&str> =
                if id.contains("5.1-codex-max") || id.contains("5.2") || id.contains("5.3") {
                    WIDELY_SUPPORTED_EFFORTS
                        .iter()
                        .copied()
                        .chain(std::iter::once("xhigh"))
                        .collect()
                } else {
                    let mut arr: Vec<&str> = WIDELY_SUPPORTED_EFFORTS.to_vec();
                    if id.contains("gpt-5") && model.release_date.as_str() >= "2025-12-04" {
                        arr.push("xhigh");
                    }
                    arr
                };
            let mut out = Map::new();
            for effort in copilot_efforts {
                out.insert(
                    effort.to_string(),
                    json!({ "reasoningEffort": effort, "reasoningSummary": "auto", "include": [INCLUDE_ENCRYPTED_REASONING] }),
                );
            }
            out
        }
        "@ai-sdk/cerebras"
        | "@ai-sdk/togetherai"
        | "@ai-sdk/xai"
        | "@ai-sdk/deepinfra"
        | "venice-ai-sdk-provider"
        | "@ai-sdk/openai-compatible" => {
            if api_id_lower.contains("north-mini-code") {
                return efforts_map(&["none", "high"], json!({ "reasoningEffort": "" }));
            }
            let mut efforts: Vec<&str> = WIDELY_SUPPORTED_EFFORTS.to_vec();
            if api_id_lower.contains("deepseek-v4") {
                efforts.push("max");
            }
            efforts_map(&efforts, json!({ "reasoningEffort": "" }))
        }
        "@ai-sdk/azure" => {
            if id == "o1-mini" {
                return Map::new();
            }
            let efforts = openai_reasoning_efforts(&id, &model.release_date);
            let mut out = Map::new();
            for e in efforts {
                out.insert(
                    e.clone(),
                    json!({ "reasoningEffort": e, "reasoningSummary": "auto", "include": [INCLUDE_ENCRYPTED_REASONING] }),
                );
            }
            out
        }
        "@ai-sdk/amazon-bedrock/mantle" | "@ai-sdk/openai" => {
            if model.provider_id == "meta" {
                let mut out = Map::new();
                for effort in ["none", "minimal", "low", "medium", "high", "xhigh"] {
                    out.insert(
                        effort.to_string(),
                        json!({ "reasoningEffort": effort, "reasoningSummary": "auto", "include": [INCLUDE_ENCRYPTED_REASONING] }),
                    );
                }
                return out;
            }
            let efforts = openai_reasoning_efforts(&api_id(model), &model.release_date);
            let mut out = Map::new();
            for e in efforts {
                out.insert(
                    e.clone(),
                    json!({ "reasoningEffort": e, "reasoningSummary": "auto", "include": [INCLUDE_ENCRYPTED_REASONING] }),
                );
            }
            out
        }
        "@ai-sdk/anthropic" | "@ai-sdk/google-vertex/anthropic" => {
            if let Some(adaptive_efforts) = &adaptive {
                let mut filtered: Vec<&str> = adaptive_efforts.clone();
                if model.provider_id == "github-copilot" {
                    if api_id_lower.contains("opus-4.7") {
                        filtered = vec!["medium"];
                    }
                    // Efforts currently supported are: low, medium, high
                    filtered.retain(|v| *v != "max" && *v != "xhigh");
                }
                let mut out = Map::new();
                for effort in &filtered {
                    let mut thinking = json!({ "type": "adaptive" });
                    if adaptive_omitted {
                        thinking["display"] = json!("summarized");
                    }
                    out.insert(
                        effort.to_string(),
                        json!({ "thinking": thinking, "effort": effort }),
                    );
                }
                return out;
            }
            if anthropic_opus45(&api_id(model)) {
                let mut out = Map::new();
                for effort in WIDELY_SUPPORTED_EFFORTS {
                    let budget = (16_000usize).min(model.limit.output as usize / 2 - 1);
                    out.insert(
                        effort.to_string(),
                        json!({ "thinking": { "type": "enabled", "budgetTokens": budget }, "effort": effort }),
                    );
                }
                return out;
            }
            serde_json::from_value(json!({
                "high": { "thinking": { "type": "enabled", "budgetTokens": (16_000).min((model.limit.output as usize) / 2 - 1) } },
                "max": { "thinking": { "type": "enabled", "budgetTokens": 31_999.min((model.limit.output as usize) - 1) } }
            })).unwrap_or_default()
        }
        "@ai-sdk/amazon-bedrock" => {
            if let Some(adaptive_efforts) = &adaptive {
                let mut out = Map::new();
                for effort in adaptive_efforts {
                    let mut config = json!({ "type": "adaptive", "maxReasoningEffort": effort });
                    if anthropic_omits_thinking(&api_id(model)) {
                        config["display"] = json!("summarized");
                    }
                    out.insert(effort.to_string(), json!({ "reasoningConfig": config }));
                }
                return out;
            }
            if api_id_lower.contains("anthropic") {
                return serde_json::from_value(json!({
                    "high": { "reasoningConfig": { "type": "enabled", "budgetTokens": 16000 } },
                    "max": { "reasoningConfig": { "type": "enabled", "budgetTokens": 31999 } }
                }))
                .unwrap_or_default();
            }
            efforts_map(
                WIDELY_SUPPORTED_EFFORTS,
                json!({ "reasoningConfig": { "type": "enabled", "maxReasoningEffort": "" } }),
            )
        }
        "@ai-sdk/google-vertex" | "@ai-sdk/google" => google_thinking_variants(model),
        "@ai-sdk/mistral" => {
            const MISTRAL_IDS: &[&str] = &[
                "mistral-small-2603",
                "mistral-small-latest",
                "mistral-medium-3.5",
                "mistral-medium-2604",
            ];
            if !MISTRAL_IDS.iter().any(|mid| api_id_lower.contains(mid)) {
                return Map::new();
            }
            efforts_map(&["high"], json!({ "reasoningEffort": "" }))
        }
        "@ai-sdk/groq" => {
            let efforts: Vec<&str> = std::iter::once("none")
                .chain(WIDELY_SUPPORTED_EFFORTS.iter().copied())
                .collect();
            efforts_map(&efforts, json!({ "reasoningEffort": "" }))
        }
        _ => Map::new(),
    }
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
