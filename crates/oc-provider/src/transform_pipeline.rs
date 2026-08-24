//! Ported dari packages/opencode/src/provider/transform.ts bagian pipeline
//! (applyCaching, unsupportedParts, message()) dan variants()/options().
//! Plus HTTP client Anthropic/OpenAI dengan SSE streaming.

use serde_json::{json, Map, Value};

use crate::messages::{Content, Message, Part, Role};
use crate::Model;

// Re-export dari transform.rs agar satu modul entry point.
pub use crate::transform::{
    max_output_tokens, openai_compatible_reasoning_efforts, openai_reasoning_efforts, schema,
    temperature, top_k, top_p,
};

fn api_npm(model: &Model) -> String {
    model.api["npm"].as_str().unwrap_or_default().to_string()
}

fn api_id(model: &Model) -> String {
    model.api["id"].as_str().unwrap_or_default().to_string()
}

/// Padanan remeda mergeDeep untuk Value (objek direkursi, sisanya replace).
pub fn merge_deep(target: Value, source: Value) -> Value {
    match (&target, &source) {
        (Value::Object(t), Value::Object(s)) => {
            let mut out = t.clone();
            for (key, sv) in s {
                match out.get(key) {
                    Some(tv) if tv.is_object() && sv.is_object() => {
                        out.insert(key.clone(), merge_deep(tv.clone(), sv.clone()));
                    }
                    _ => {
                        out.insert(key.clone(), source.clone());
                    }
                }
            }
            Value::Object(out)
        }
        _ => source,
    }
}

/// Ported from: transform.ts:359-408 (applyCaching)
pub fn apply_caching(mut msgs: Vec<Message>, model: &Model) -> Vec<Message> {
    let system: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::System)
        .take(2)
        .map(|(i, _)| i)
        .collect();
    let total = msgs.len();
    let final_msgs: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role != Role::System)
        .skip(total.saturating_sub(2))
        .map(|(i, _)| i)
        .collect();

    let provider_options = json!({
        "anthropic": { "cacheControl": { "type": "ephemeral" } },
        "openrouter": { "cacheControl": { "type": "ephemeral" } },
        "bedrock": { "cachePoint": { "type": "default" } },
        "openaiCompatible": { "cache_control": { "type": "ephemeral" } },
        "copilot": { "copilot_cache_control": { "type": "ephemeral" } },
        "alibaba": { "cacheControl": { "type": "ephemeral" } },
    });

    // unique indices (system + final bisa overlap)
    let mut seen = std::collections::BTreeSet::new();
    let mut targets = Vec::new();
    for i in system.into_iter().chain(final_msgs) {
        if seen.insert(i) {
            targets.push(i);
        }
    }

    let use_message_level = model.provider_id == "anthropic"
        || model.provider_id.contains("bedrock")
        || api_npm(model) == "@ai-sdk/amazon-bedrock";

    for idx in targets {
        let msg = &mut msgs[idx];
        if !use_message_level
            && matches!(msg.content, Content::Parts(ref parts) if !parts.is_empty())
        {
            // content-level options menunggu Part.provider_options field (Sprint 9)
            continue;
        }
        let opts = msg.provider_options.get_or_insert_with(Map::new);
        *opts = merge_deep_json_map(opts.clone(), provider_options.as_object().unwrap().clone());
    }

    msgs
}

fn merge_deep_json_map(
    target: Map<String, Value>,
    source: Map<String, Value>,
) -> Map<String, Value> {
    let merged = merge_deep(Value::Object(target), Value::Object(source));
    merged.as_object().cloned().unwrap_or_default()
}

/// Ported from: transform.ts:410-446 (unsupportedParts)
pub fn unsupported_parts(mut msgs: Vec<Message>, model: &Model) -> Vec<Message> {
    for msg in &mut msgs {
        if msg.role != Role::User {
            continue;
        }
        let Content::Parts(parts) = &mut msg.content else {
            continue;
        };
        *parts = parts
            .iter()
            .map(|part| match part {
                Part::Image { image } => {
                    // empty base64 check
                    if image.starts_with("data:") {
                        if let Some(comma_pos) = image.find(";base64,") {
                            let data = &image[comma_pos + 8..];
                            if data.is_empty() {
                                return Part::Text {
                                    text: "ERROR: Image file is empty or corrupted. Please provide a valid image.".into(),
                                };
                            }
                        }
                    }
                    let mime = image.split(';').next().unwrap_or("").replace("data:", "");
                    let modality = mime_to_modality(&mime);
                    match modality {
                        Some(m) if supports_input(model, m) => part.clone(),
                        Some(m) => Part::Text {
                            text: format!("ERROR: Cannot read {m} (this model does not support {m} input). Inform the user."),
                        },
                        None => part.clone(),
                    }
                }
                Part::File { media_type, filename } => {
                    let modality = mime_to_modality(media_type);
                    match modality {
                        Some(m) if supports_input(model, m) => part.clone(),
                        Some(m) => {
                            let name = filename.as_deref().map(|f| format!("\"{f}\"")).unwrap_or(m.to_string());
                            Part::Text {
                                text: format!("ERROR: Cannot read {name} (this model does not support {m} input). Inform the user."),
                            }
                        }
                        None => part.clone(),
                    }
                }
                other => other.clone(),
            })
            .collect();
    }
    msgs
}

fn mime_to_modality(mime: &str) -> Option<&'static str> {
    if mime.starts_with("image/") {
        Some("image")
    } else if mime.starts_with("audio/") {
        Some("audio")
    } else if mime.starts_with("video/") {
        Some("video")
    } else if mime == "application/pdf" {
        Some("pdf")
    } else {
        None
    }
}

/// capabilities.input[modality] — port Rust menyimpan modalities di options.
fn supports_input(model: &Model, modality: &str) -> bool {
    model
        .options
        .get("input_modalities")
        .and_then(|v| v.as_array())
        .map(|list| list.iter().any(|item| item.as_str() == Some(modality)))
        .unwrap_or(true) // default: semua didukung bila tidak dispesifikasi
}

/// Ported from: transform.ts:448-464 (mapProviderOptions)
pub fn map_provider_options(
    mut msgs: Vec<Message>,
    transform: impl Fn(&mut Map<String, Value>),
) -> Vec<Message> {
    for msg in &mut msgs {
        if let Some(opts) = &mut msg.provider_options {
            transform(opts);
        }
        if let Content::Parts(parts) = &mut msg.content {
            for part in parts.iter_mut() {
                if part.is_approval() {
                    continue;
                }
                // part-level provider_options menunggu Sprint 9
            }
        }
    }
    msgs
}

/// Ported from: transform.ts:466-519 (message) — pipeline utama.
pub fn message(
    mut msgs: Vec<Message>,
    model: &Model,
    options: &Map<String, Value>,
) -> Vec<Message> {
    msgs = unsupported_parts(msgs, model);

    // normalize_messages ada di crate yang sama
    let mut msgs = crate::transform_messages::normalize_messages(msgs, model);

    let npm = api_npm(model);
    let uses_anthropic_auto_caching = options.contains_key("cacheControl")
        && (npm == "@ai-sdk/anthropic" || npm == "@ai-sdk/google-vertex/anthropic");

    let is_anthropic_family = model.provider_id == "anthropic"
        || model.provider_id == "google-vertex-anthropic"
        || api_id(model).contains("anthropic")
        || api_id(model).contains("claude")
        || model.id.contains("anthropic")
        || model.id.contains("claude")
        || npm == "@ai-sdk/anthropic"
        || npm == "@ai-sdk/alibaba";

    if is_anthropic_family && npm != "@ai-sdk/gateway" && !uses_anthropic_auto_caching {
        msgs = apply_caching(msgs, model);
    }

    // sdkKey remap
    if let Some(key) = crate::transform_messages::sdk_key(&npm) {
        if key != model.provider_id {
            msgs = map_provider_options(msgs, |opts| {
                if !opts.contains_key(&model.provider_id) {
                    return;
                }
                let value = opts.remove(&model.provider_id).unwrap();
                opts.insert(key.to_string(), value);
            });
        }
    }

    // strip itemId saat store != true
    if !options
        .get("store")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if let Some(key) = crate::transform_messages::sdk_key(&npm) {
            if [
                "@ai-sdk/openai",
                "@ai-sdk/azure",
                "@ai-sdk/amazon-bedrock/mantle",
                "@ai-sdk/github-copilot",
            ]
            .contains(&npm.as_str())
            {
                msgs = map_provider_options(msgs, |opts| {
                    let Some(entry) = opts.get_mut(key) else {
                        return;
                    };
                    let Value::Object(map) = entry else { return };
                    map.remove("itemId");
                });
            }
        }
    }

    msgs
}
