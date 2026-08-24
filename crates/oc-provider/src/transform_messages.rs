//! Ported dari packages/opencode/src/provider/transform.ts (utuh).
//!
//! Cabang yang membandingkan `model.api.npm` direplikasi lewat string npm
//! aslinya agar perilaku identik; konversi ke enum SDK-family dilakukan di
//! `sdk_family()`.

use serde_json::{json, Map, Value};

use crate::messages::{Content, Message, Part, Role, ToolOutput};
use crate::Model;

/// Ported from: transform.ts:25-27 (sanitizeSurrogates)
/// Unpaired UTF-16 surrogates → U+FFFD. Di Rust string selalu valid UTF-8,
/// sehingga padanannya: ganti setiap code point di rentang surrogate dengan
/// U+FFFD (tidak mungkin ada pasangan valid di Rust String).
pub fn sanitize_surrogates(content: &str) -> String {
    if !content
        .chars()
        .any(|c| (0xD800..=0xDFFF).contains(&(c as u32)))
    {
        return content.to_string();
    }
    content
        .chars()
        .map(|c| {
            if (0xD800..=0xDFFF).contains(&(c as u32)) {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

/// Ported from: transform.ts:29-39 (isKimiFamily)
pub fn is_kimi_family(model: &Model) -> bool {
    let ids = [&model.provider_id, model.api["id"].as_str().unwrap_or("")];
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

/// Ported from: transform.ts:42-98 (sdkKey — tabel pemetaan npm → sdk key)
pub fn sdk_key(npm: &str) -> Option<&'static str> {
    match npm {
        "@ai-sdk/github-copilot" => Some("copilot"),
        "@ai-sdk/azure" => Some("azure"),
        "@ai-sdk/openai" => Some("openai"),
        "@ai-sdk/amazon-bedrock/mantle" => Some("openai"),
        "@ai-sdk/amazon-bedrock" => Some("bedrock"),
        "@ai-sdk/anthropic" | "@ai-sdk/google-vertex/anthropic" => Some("anthropic"),
        "@ai-sdk/google-vertex" => Some("vertex"),
        "@ai-sdk/google" => Some("google"),
        "@ai-sdk/alibaba" => Some("alibaba"),
        "@ai-sdk/cerebras" => Some("cerebras"),
        "@ai-sdk/cohere" => Some("cohere"),
        "@ai-sdk/deepinfra" => Some("deepinfra"),
        "@ai-sdk/groq" => Some("groq"),
        "@ai-sdk/mistral" => Some("mistral"),
        "@ai-sdk/perplexity" => Some("perplexity"),
        "@ai-sdk/togetherai" => Some("togetherai"),
        "@ai-sdk/vercel" => Some("vercel"),
        "@ai-sdk/xai" => Some("xai"),
        "venice-ai-sdk-provider" => Some("venice"),
        "@ai-sdk/gateway" => Some("gateway"),
        "@openrouter/ai-sdk-provider" => Some("openrouter"),
        "merge-gateway-ai-sdk-provider" => Some("mergeGateway"),
        "ai-gateway-provider" => Some("openaiCompatible"),
        _ => None,
    }
}

fn api_npm(model: &Model) -> String {
    model.api["npm"].as_str().unwrap_or_default().to_string()
}

fn api_id(model: &Model) -> String {
    model.api["id"].as_str().unwrap_or_default().to_string()
}

/// Ported from: transform.ts:100-357 (normalizeMessages)
pub fn normalize_messages(msgs: Vec<Message>, model: &Model) -> Vec<Message> {
    let mut msgs = sanitize_all(msgs);

    let npm = api_npm(model);
    // Anthropic / Bedrock: buang pesan kosong + part teks/reasoning kosong
    if npm == "@ai-sdk/anthropic" || npm == "@ai-sdk/amazon-bedrock" {
        msgs = msgs
            .into_iter()
            .filter_map(|msg| match msg.content.clone() {
                Content::Text(text) => {
                    if text.is_empty() {
                        None
                    } else {
                        Some(msg)
                    }
                }
                Content::Parts(parts) => {
                    let mut parts: Vec<Part> = parts;
                    parts.retain(|part| match part {
                        Part::Text { text } => !text.is_empty(),
                        Part::Reasoning {
                            text,
                            signature,
                            redacted_data,
                        } => {
                            !text.trim().is_empty()
                                || signature.as_deref().map(str::is_empty) == Some(false)
                                || signature.is_some()
                                || redacted_data.is_some()
                        }
                        _ => true,
                    });
                    if parts.is_empty() {
                        None
                    } else {
                        Some(msg)
                    }
                }
            })
            .collect();
    }

    // Claude: scrub tool call ID ke [a-zA-Z0-9_-]
    if api_id(model).contains("claude") {
        for msg in &mut msgs {
            if let Some(parts) = msg.parts_mut() {
                for part in parts.iter_mut() {
                    if matches!(part, Part::ToolCall { .. }) {
                        if let Part::ToolCall { tool_call_id, .. } = part {
                            *tool_call_id = scrub_claude(tool_call_id);
                        }
                    } else if let Part::ToolResult { tool_call_id, .. } = part {
                        *tool_call_id = scrub_claude(tool_call_id);
                    }
                }
            }
        }
    }

    // Mistral family: scrub 9-char alphanumeric + sisip "Done." antara tool→user
    let id_lower = api_id(model).to_lowercase();
    let is_mistral = model.provider_id == "mistral"
        || ["mistral", "devstral", "codestral", "pixtral", "mixtral"]
            .iter()
            .any(|f| id_lower.contains(f));
    if is_mistral {
        let mut result: Vec<Message> = Vec::new();
        for i in 0..msgs.len() {
            let next_role = msgs.get(i + 1).map(|m| m.role);
            let mut msg = msgs[i].clone();
            if let Some(parts) = msg.parts_mut() {
                for part in parts.iter_mut() {
                    if let Part::ToolCall { tool_call_id, .. } = part {
                        *tool_call_id = scrub_mistral(tool_call_id);
                    } else if let Part::ToolResult { tool_call_id, .. } = part {
                        *tool_call_id = scrub_mistral(tool_call_id);
                    }
                }
            }
            let was_tool = msg.role == Role::Tool;
            result.push(msg);
            if was_tool && next_role == Some(Role::User) {
                result.push(Message::parts(
                    Role::Assistant,
                    vec![Part::Text {
                        text: "Done.".into(),
                    }],
                ));
            }
        }
        return finish_interleaved(result, model);
    }

    // DeepSeek: assistant wajib punya reasoning part
    if id_lower.contains("deepseek") {
        for msg in &mut msgs {
            if msg.role != Role::Assistant {
                continue;
            }
            match &mut msg.content {
                Content::Parts(parts) => {
                    if !parts.iter().any(|p| matches!(p, Part::Reasoning { .. })) {
                        parts.push(Part::Reasoning {
                            text: String::new(),
                            signature: None,
                            redacted_data: None,
                        });
                    }
                }
                Content::Text(text) => {
                    let text = std::mem::take(text);
                    msg.content = Content::Parts(vec![
                        Part::Text { text },
                        Part::Reasoning {
                            text: String::new(),
                            signature: None,
                            redacted_data: None,
                        },
                    ]);
                }
            }
        }
    }

    finish_interleaved(msgs, model)
}

/// Ported dari cabang interleaved (transform.ts:322-354): pindahkan reasoning
/// ke providerOptions.openaiCompatible.<field>.
fn finish_interleaved(mut msgs: Vec<Message>, model: &Model) -> Vec<Message> {
    let interleaved = &model.capabilities;
    let _ = interleaved;
    // capabilities.interleaved berada di Model.capabilities pada TS; port Rust
    // menyimpannya di options["interleaved"] sebagai {"field": "..."} bila ada.
    let Some(interleaved_value) = model.options.get("interleaved") else {
        return msgs;
    };
    let Some(field) = interleaved_value["field"].as_str() else {
        return msgs;
    };
    if api_npm(model) == "@openrouter/ai-sdk-provider" {
        return msgs;
    }
    for msg in &mut msgs {
        if msg.role != Role::Assistant {
            continue;
        }
        let Content::Parts(parts) = &mut msg.content else {
            continue;
        };
        let reasoning_text: String = parts
            .iter()
            .filter_map(|p| match p {
                Part::Reasoning { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        parts.retain(|p| !matches!(p, Part::Reasoning { .. }));

        let opts = msg.provider_options.get_or_insert_with(Map::new);
        let entry = opts
            .entry("openaiCompatible".to_string())
            .or_insert_with(|| json!({}));
        if let Value::Object(map) = entry {
            map.insert(field.to_string(), Value::String(reasoning_text));
        }
    }
    msgs
}

fn sanitize_all(mut msgs: Vec<Message>) -> Vec<Message> {
    for msg in &mut msgs {
        match &mut msg.content {
            Content::Text(text) => {
                *text = sanitize_surrogates(text);
            }
            Content::Parts(parts) => {
                for part in parts.iter_mut() {
                    if let Some(text) = part.text_mut() {
                        *text = sanitize_surrogates(text);
                    }
                    if let Some(output) = part.tool_result_output_mut() {
                        sanitize_tool_result(output);
                    }
                }
            }
        }
    }
    msgs
}

/// Ported dari closure sanitizeToolResultOutput (transform.ts:106-119).
fn sanitize_tool_result(output: &mut ToolOutput) {
    match output {
        ToolOutput::Text(text) | ToolOutput::ErrorText(text) => {
            *text = sanitize_surrogates(text);
        }
        ToolOutput::Content(items) => {
            for item in items.iter_mut() {
                if let crate::messages::OutputItem::Text(text) = item {
                    *text = sanitize_surrogates(text);
                }
            }
        }
        ToolOutput::Other(_) => {}
    }
}

fn scrub_claude(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn scrub_mistral(id: &str) -> String {
    let cleaned: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let mut truncated: String = cleaned.chars().take(9).collect();
    while truncated.len() < 9 {
        truncated.push('0');
    }
    truncated
}
