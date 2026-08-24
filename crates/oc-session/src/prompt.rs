//! Ported dari packages/opencode/src/session/prompt.ts (prompt processing
//! loop) — subset inti: resolve tools → call provider → parse tool calls →
//! execute → feed back → repeat.
//!
//! Subagent spawning dan interrupt/cancellation menyusul (10b/10c).

use std::sync::Arc;

use serde_json::{json, Value};

use crate::model::{
    AssistantMessage, BaseIds, CacheReadWrite, Part, SessionPath, TimeWithCompletion, TokenUsage,
    UserOrAssistant,
};
use crate::store::SessionStore;

/// Trait provider agar loop testable dengan mock.
pub trait ProviderClient: Send + Sync {
    /// Kirim messages+tools → kembalikan response JSON (non-streaming).
    fn send(
        &self,
        model_id: &str,
        system: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<Value, String>;
}

/// Event yang dipublish saat loop berjalan.
#[derive(Debug, Clone)]
pub enum LoopEvent {
    PartUpdated {
        session_id: String,
        message_id: String,
        part_id: String,
    },
    MessageCompleted {
        session_id: String,
        message_id: String,
    },
    ToolExecuted {
        call_id: String,
        tool: String,
    },
}

pub type EventSender = Arc<dyn Fn(LoopEvent) + Send + Sync>;

/// Tool executor trait — di-inject dari oc-tool.
pub trait ToolExecutor: Send + Sync {
    fn execute(
        &self,
        tool_name: &str,
        args: &Value,
        ctx: &ToolContext,
    ) -> Result<crate::tool_result::ToolOutput, String>;
}

pub type AskCallback = Box<dyn Fn(&str, Vec<String>) -> Result<(), String> + Send + Sync>;

/// Context yang diteruskan ke tiap tool eksekusi.
pub struct ToolContext {
    pub session_id: String,
    pub message_id: String,
    pub agent: String,
    pub directory: std::path::PathBuf,
    pub worktree: std::path::PathBuf,
    pub ask: AskCallback,
}

/// Loop input.
pub struct PromptLoopInput<'a> {
    pub session_id: &'a str,
    pub parent_message_id: &'a str,
    pub agent: &'a str,
    pub model_provider_id: &'a str,
    pub model_id: &'a str,
    pub system: &'a str,
    pub directory: &'a std::path::Path,
    pub worktree: &'a std::path::Path,
    pub max_tokens: usize,
    pub max_iterations: usize,
    /// Cancellation token untuk interrupt loop (opsional).
    pub cancellation: Option<crate::cancellation::CancellationToken>,
}

/// Hasil akhir loop.
pub struct LoopResult {
    pub assistant_message_id: String,
    pub output_text: String,
    pub tokens: TokenUsageResult,
    pub cost: f64,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsageResult {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Ported dari prompt.ts — loop utama.
pub fn run_prompt_loop(
    store: &SessionStore,
    provider: &dyn ProviderClient,
    executor: &dyn ToolExecutor,
    event_sender: &EventSender,
    input: &PromptLoopInput,
    conversation_messages: &[Value],
    tools: &[Value],
) -> Result<LoopResult, String> {
    let mut iteration = 0usize;
    let mut all_parts: Vec<Part> = Vec::new();
    let mut output_text = String::new();
    let mut total_tokens = TokenUsageResult::default();
    let cost = 0.0f64;
    let mut finish_reason = None;

    // Buat assistant message ID
    let assistant_msg_id = format!("msg_{}", new_id());
    let now = now_millis();

    // Simpan assistant message awal
    let assistant = crate::model::UserOrAssistant::Assistant(AssistantMessage {
        id: assistant_msg_id.clone(),
        session_id: input.session_id.to_string(),
        time: TimeWithCompletion {
            created: now,
            completed: None,
        },
        error: None,
        parent_id: input.parent_message_id.to_string(),
        model_id: input.model_id.to_string(),
        provider_id: input.model_provider_id.to_string(),
        mode: "primary".to_string(),
        agent: input.agent.to_string(),
        path: SessionPath {
            cwd: input.directory.to_string_lossy().into_owned(),
            root: input.worktree.to_string_lossy().into_owned(),
        },
        summary: None,
        cost: 0.0,
        tokens: TokenUsage::default(),
        structured: None,
        variant: None,
        finish: None,
    });
    store
        .append_message(&assistant)
        .map_err(|e| e.to_string())?;

    let mut working_messages: Vec<Value> = conversation_messages.to_vec();

    loop {
        if iteration >= input.max_iterations {
            break;
        }
        if let Some(token) = &input.cancellation {
            if token.is_cancelled() {
                finish_reason = Some("aborted".to_string());
                break;
            }
        }
        iteration += 1;

        // 1. Kirim ke provider
        let response = provider.send(input.model_id, input.system, &working_messages, tools)?;

        // 2. Parse response
        let choice = response["choices"][0].clone();
        let message = choice["message"].clone();
        // finish_reason selalu diambil dari iterasi terakhir
        finish_reason = choice["finish_reason"].as_str().map(String::from);

        // 3. Extract text content
        let content_text = message["content"].as_str().unwrap_or("").to_string();

        // 4. Extract tool calls
        let tool_calls: Vec<Value> = message["tool_calls"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        // 5. Update token usage dari response
        if let Some(usage) = response.get("usage") {
            total_tokens.input += usage["prompt_tokens"].as_u64().unwrap_or(0);
            total_tokens.output += usage["completion_tokens"].as_u64().unwrap_or(0);
            total_tokens.total = total_tokens.input + total_tokens.output;
        }

        // 6. Jika tidak ada tool calls → final response
        if tool_calls.is_empty() {
            output_text.push_str(&content_text);

            // simpan text part
            let part_id = format!("prt_{}", new_id());
            let part = Part::Text {
                ids: BaseIds {
                    id: part_id.clone(),
                    session_id: input.session_id.to_string(),
                    message_id: assistant_msg_id.clone(),
                },
                text: content_text.clone(),
                synthetic: None,
                ignored: None,
                time: None,
                metadata: None,
            };
            store
                .write_part(input.session_id, &assistant_msg_id, &part)
                .map_err(|e| e.to_string())?;
            event_sender(LoopEvent::PartUpdated {
                session_id: input.session_id.to_string(),
                message_id: assistant_msg_id.clone(),
                part_id,
            });
            all_parts.push(part);

            // tambahkan assistant response ke working messages
            working_messages.push(json!({
                "role": "assistant",
                "content": content_text
            }));
            break;
        }

        // 7. Simpan assistant message dengan tool_calls ke working messages
        let mut assistant_content = Vec::new();
        if !content_text.is_empty() {
            assistant_content.push(json!({"type": "text", "text": content_text}));
            output_text.push_str(&content_text);
        }

        // simpan tool call parts + jalankan tools
        for call in &tool_calls {
            let call_id = call["id"].as_str().unwrap_or("").to_string();
            let tool_name = call["function"]["name"].as_str().unwrap_or("").to_string();
            let arguments_raw = call["function"]["arguments"].as_str().unwrap_or("{}");
            let arguments: Value =
                serde_json::from_str(arguments_raw).unwrap_or_else(|_| json!({}));

            // simpan tool call part
            let part_id = format!("prt_{}", new_id());
            let part = Part::Tool {
                ids: BaseIds {
                    id: part_id.clone(),
                    session_id: input.session_id.to_string(),
                    message_id: assistant_msg_id.clone(),
                },
                call_id: call_id.clone(),
                tool: tool_name.clone(),
                state: crate::model::ToolState::Pending {
                    input: Default::default(),
                    raw: arguments_raw.to_string(),
                },
                metadata: None,
            };
            store
                .write_part(input.session_id, &assistant_msg_id, &part)
                .map_err(|e| e.to_string())?;
            event_sender(LoopEvent::PartUpdated {
                session_id: input.session_id.to_string(),
                message_id: assistant_msg_id.clone(),
                part_id: part_id.clone(),
            });

            // eksekusi tool via executor
            let tool_ctx = ToolContext {
                session_id: input.session_id.to_string(),
                message_id: assistant_msg_id.clone(),
                agent: input.agent.to_string(),
                directory: input.directory.to_path_buf(),
                worktree: input.worktree.to_path_buf(),
                ask: Box::new(|_perm, _patterns| Ok(())), // auto-allow untuk sekarang
            };

            let result = executor.execute(&tool_name, &arguments, &tool_ctx);
            let (result_content, _is_error) = match result {
                Ok(output) => match output {
                    crate::tool_result::ToolOutput::Text(text) => (text, false),
                },
                Err(error) => (format!("Error: {error}"), true),
            };

            event_sender(LoopEvent::ToolExecuted {
                call_id: call_id.clone(),
                tool: tool_name.clone(),
            });

            // tambahkan tool result ke assistant_content
            assistant_content.push(json!({
                "type": "tool_use",
                "id": call_id,
                "name": tool_name,
                "input": arguments
            }));

            // tambahkan tool result sebagai user message
            working_messages.push(json!({
                "role": "assistant",
                "content": assistant_content.clone()
            }));
            working_messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_call_id": call_id,
                    "content": result_content
                }]
            }));

            // clear assistant_content untuk tool call berikutnya dalam batch sama
            assistant_content.clear();
        }

        // jika hanya ada satu tool call dan sudah diproses, lanjut iterasi
        if !tool_calls.is_empty() && assistant_content.is_empty() {
            continue;
        }
    }

    // update assistant message dengan completed time + tokens
    let updated_assistant = UserOrAssistant::Assistant(AssistantMessage {
        id: assistant_msg_id.clone(),
        session_id: input.session_id.to_string(),
        time: TimeWithCompletion {
            created: now,
            completed: Some(now_millis()),
        },
        error: None,
        parent_id: input.parent_message_id.to_string(),
        model_id: input.model_id.to_string(),
        provider_id: input.model_provider_id.to_string(),
        mode: "primary".to_string(),
        agent: input.agent.to_string(),
        path: SessionPath {
            cwd: input.directory.to_string_lossy().into_owned(),
            root: input.worktree.to_string_lossy().into_owned(),
        },
        summary: None,
        cost,
        tokens: TokenUsage {
            total: Some(total_tokens.total as f64),
            input: total_tokens.input as f64,
            output: total_tokens.output as f64,
            reasoning: total_tokens.reasoning as f64,
            cache: CacheReadWrite {
                read: total_tokens.cache_read as f64,
                write: total_tokens.cache_write as f64,
            },
        },
        structured: None,
        variant: None,
        finish: finish_reason.clone(),
    });
    store
        .append_message(&updated_assistant)
        .map_err(|e| e.to_string())?;
    event_sender(LoopEvent::MessageCompleted {
        session_id: input.session_id.to_string(),
        message_id: assistant_msg_id.clone(),
    });

    Ok(LoopResult {
        assistant_message_id: assistant_msg_id,
        output_text,
        tokens: total_tokens,
        cost,
        finish_reason,
    })
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn counter() -> &'static std::sync::Mutex<u64> {
    static COUNTER: std::sync::OnceLock<std::sync::Mutex<u64>> = std::sync::OnceLock::new();
    COUNTER.get_or_init(|| std::sync::Mutex::new(0))
}

fn new_id() -> String {
    let mut guard = counter().lock().unwrap();
    *guard += 1;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("{millis:x}{:04x}", *guard)
}
