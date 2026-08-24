//! HTTP client untuk Anthropic Messages API dan OpenAI Chat Completions.
//!
//! SSE streaming diimplementasikan dengan `ureq` blocking reader — cukup
//! untuk tool-call loop sinkron; async/tokio menyusul saat session loop
//! (Sprint 10) butuh concurrency.

use serde_json::{json, Map, Value};

use crate::Model;

fn api_id(model: &Model) -> String {
    model.api["id"].as_str().unwrap_or_default().to_string()
}

#[derive(Debug)]
pub enum ProviderHttpError {
    Auth(String),
    RateLimit(String),
    BadRequest(String),
    Server(String),
    Network(String),
}

impl std::fmt::Display for ProviderHttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auth(msg) => write!(f, "auth error: {msg}"),
            Self::RateLimit(msg) => write!(f, "rate limit: {msg}"),
            Self::BadRequest(msg) => write!(f, "bad request: {msg}"),
            Self::Server(msg) => write!(f, "server error: {msg}"),
            Self::Network(msg) => write!(f, "network error: {msg}"),
        }
    }
}

fn map_ureq_error(error: ureq::Error) -> ProviderHttpError {
    match error {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            match code {
                401 | 403 => ProviderHttpError::Auth(body),
                429 => ProviderHttpError::RateLimit(body),
                400..=422 => ProviderHttpError::BadRequest(body),
                _ => ProviderHttpError::Server(body),
            }
        }
        other => ProviderHttpError::Network(other.to_string()),
    }
}

/// Resolusi API key: auth.json → env var.
pub fn resolve_api_key(model: &Model) -> Option<String> {
    // env var per provider (urutan sesuai source TS provider.ts)
    let candidates: &[&str] = match model.provider_id.as_str() {
        "anthropic" => &["ANTHROPIC_API_KEY"],
        "openai" => &["OPENAI_API_KEY"],
        "google" => &["GOOGLE_GENERATIVE_AI_API_KEY", "GEMINI_API_KEY"],
        _ => &[],
    };
    for key in candidates {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Event dari SSE stream.
#[derive(Debug, Clone)]
pub enum SseEvent {
    Data(Value),
    Done,
}

/// Parse satu baris SSE → event bila line adalah data.
pub fn parse_sse_line(line: &str) -> Option<SseEvent> {
    let data = line
        .strip_prefix("data: ")
        .or_else(|| line.strip_prefix("data:"))?;
    if data.trim() == "[DONE]" {
        return Some(SseEvent::Done);
    }
    serde_json::from_str::<Value>(data.trim())
        .ok()
        .map(SseEvent::Data)
}

/// Kumpulkan seluruh SSE events dari string body.
pub fn parse_sse_body(body: &str) -> Vec<SseEvent> {
    body.lines().filter_map(parse_sse_line).collect()
}

// --- Anthropic Messages API ---

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Build request body untuk Anthropic Messages API.
pub fn anthropic_request_body(
    model: &Model,
    system: &str,
    messages: &[Value],
    tools: &[Value],
    max_tokens: usize,
    options: &Map<String, Value>,
) -> Value {
    let mut body = json!({
        "model": api_id(model),
        "max_tokens": max_tokens,
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }

    // temperature / top_p / top_k dari transform.rs
    if let Some(temp) = crate::transform::temperature(model) {
        body["temperature"] = json!(temp);
    }
    if let Some(top_p) = crate::transform::top_p(model) {
        body["top_p"] = json!(top_p);
    }
    if let Some(top_k) = crate::transform::top_k(model) {
        body["top_k"] = json!(top_k);
    }

    // extra options dari options()/variants()
    for (key, value) in options {
        if !key.is_empty() && !["toolStreaming"].contains(&key.as_str()) {
            body[key.as_str()] = value.clone();
        }
    }
    body
}

/// Kirim request non-streaming ke Anthropic API → response JSON.
pub fn anthropic_send(
    api_key: &str,
    body: &Value,
    base_url: Option<&str>,
) -> Result<Value, ProviderHttpError> {
    let base = base_url.unwrap_or(ANTHROPIC_BASE_URL);
    let url = format!("{base}/v1/messages");
    let response = ureq::post(&url)
        .set("x-api-key", api_key)
        .set("anthropic-version", ANTHROPIC_VERSION)
        .set("content-type", "application/json")
        .send_string(&serde_json::to_string(body).unwrap_or_default())
        .map_err(map_ureq_error)?;
    let text = response
        .into_string()
        .map_err(|e| ProviderHttpError::Network(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| ProviderHttpError::Network(e.to_string()))
}

/// Kirim streaming request ke Anthropic API → kembalikan raw SSE body text.
pub fn anthropic_send_streaming(
    api_key: &str,
    body: &Value,
    base_url: Option<&str>,
) -> Result<String, ProviderHttpError> {
    let base = base_url.unwrap_or(ANTHROPIC_BASE_URL);
    let url = format!("{base}/v1/messages");
    let mut stream_body = body.clone();
    stream_body["stream"] = json!(true);
    let response = ureq::post(&url)
        .set("x-api-key", api_key)
        .set("anthropic-version", ANTHROPIC_VERSION)
        .set("content-type", "application/json")
        .send_string(&serde_json::to_string(&stream_body).unwrap_or_default())
        .map_err(map_ureq_error)?;
    let mut text = String::new();
    response
        .into_reader()
        .read_to_string(&mut text)
        .map_err(|e| ProviderHttpError::Network(e.to_string()))?;
    Ok(text)
}

// --- OpenAI Chat Completions API ---

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// Build request body untuk OpenAI Chat Completions.
pub fn openai_request_body(
    model: &Model,
    system: Option<&str>,
    messages: &[Value],
    tools: &[Value],
    options: &Map<String, Value>,
) -> Value {
    let mut msgs_json = Vec::new();
    if let Some(system_text) = system {
        if !system_text.is_empty() {
            msgs_json.push(json!({ "role": "system", "content": system_text }));
        }
    }
    msgs_json.extend(messages.iter().cloned());

    let mut body = json!({
        "model": api_id(model),
        "messages": msgs_json,
    });
    if !tools.is_empty() {
        body["tools"] = json!(tools);
    }

    if let Some(temp) = crate::transform::temperature(model) {
        body["temperature"] = json!(temp);
    }
    if let Some(top_p) = crate::transform::top_p(model) {
        body["top_p"] = json!(top_p);
    }

    for (key, value) in options {
        if !key.is_empty() {
            body[key.as_str()] = value.clone();
        }
    }
    body
}

/// Kirim non-streaming ke OpenAI API.
pub fn openai_send(
    api_key: &str,
    body: &Value,
    base_url: Option<&str>,
) -> Result<Value, ProviderHttpError> {
    let base = base_url.unwrap_or(OPENAI_BASE_URL);
    let url = format!("{base}/chat/completions");
    let response = ureq::post(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_string(&serde_json::to_string(body).unwrap_or_default())
        .map_err(map_ureq_error)?;
    let text = response
        .into_string()
        .map_err(|e| ProviderHttpError::Network(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| ProviderHttpError::Network(e.to_string()))
}

/// Kirim streaming ke OpenAI API → raw SSE body.
pub fn openai_send_streaming(
    api_key: &str,
    body: &Value,
    base_url: Option<&str>,
) -> Result<String, ProviderHttpError> {
    let base = base_url.unwrap_or(OPENAI_BASE_URL);
    let url = format!("{base}/chat/completions");
    let mut stream_body = body.clone();
    stream_body["stream"] = json!(true);
    let response = ureq::post(&url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Content-Type", "application/json")
        .send_string(&serde_json::to_string(&stream_body).unwrap_or_default())
        .map_err(map_ureq_error)?;
    let mut text = String::new();
    response
        .into_reader()
        .read_to_string(&mut text)
        .map_err(|e| ProviderHttpError::Network(e.to_string()))?;
    Ok(text)
}

/// Ekstrak teks + tool calls dari Anthropic Messages response.
pub fn parse_anthropic_response(response: &Value) -> (String, Vec<Value>) {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    if let Some(content) = response.get("content").and_then(Value::as_array) {
        for block in content {
            match block.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if let Some(t) = block.get("text").and_then(Value::as_str) {
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    tool_calls.push(block.clone());
                }
                _ => {}
            }
        }
    }
    (text, tool_calls)
}

/// Ekstrak teks + tool calls dari OpenAI Chat Completions response.
pub fn parse_openai_response(response: &Value) -> (String, Vec<Value>) {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    if let Some(choices) = response.get("choices").and_then(Value::as_array) {
        if let Some(first) = choices.first() {
            if let Some(message) = first.get("message") {
                if let Some(content) = message.get("content").and_then(Value::as_str) {
                    text = content.to_string();
                }
                if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
                    tool_calls = calls.clone();
                }
            }
        }
    }
    (text, tool_calls)
}
