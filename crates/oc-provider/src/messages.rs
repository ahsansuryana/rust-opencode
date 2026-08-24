//! Representasi ModelMessage (AI SDK "ai" package) yang dipakai transform.ts.
//! Minimal tapi cukup untuk seluruh cabang transform.ts; akan diselaraskan
//! dengan oc-session data model (Sprint 9).

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }
}

/// Ported dari ToolResultPart.output: text | error-text | content | ...
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutput {
    Text(String),
    ErrorText(String),
    Content(Vec<OutputItem>),
    Other(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputItem {
    Text(String),
    Other(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Text {
        text: String,
    },
    Reasoning {
        text: String,
        /// providerOptions.anthropic.signature / bedrock.signature
        signature: Option<String>,
        redacted_data: Option<String>,
    },
    ToolCall {
        tool_call_id: String,
    },
    ToolResult {
        tool_call_id: String,
        output: ToolOutput,
    },
    Image {
        /// data URI atau referensi
        image: String,
    },
    File {
        media_type: String,
        filename: Option<String>,
    },
    ApprovalRequest,
    ApprovalResponse,
    Other,
}

impl Part {
    /// Ambil teks bila part bertipe text/reasoning (untuk sanitize).
    pub fn text_mut(&mut self) -> Option<&mut String> {
        match self {
            Part::Text { text } => Some(text),
            Part::Reasoning { text, .. } => Some(text),
            _ => None,
        }
    }

    pub fn tool_result_output_mut(&mut self) -> Option<&mut ToolOutput> {
        match self {
            Part::ToolResult { output, .. } => Some(output),
            _ => None,
        }
    }

    pub fn is_approval(&self) -> bool {
        matches!(self, Part::ApprovalRequest | Part::ApprovalResponse)
    }
}

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Parts(Vec<Part>),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Content,
    pub provider_options: Option<Map<String, Value>>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Message {
            role: Role::System,
            content: Content::Text(text.into()),
            provider_options: None,
        }
    }

    pub fn parts(role: Role, parts: Vec<Part>) -> Self {
        Message {
            role,
            content: Content::Parts(parts),
            provider_options: None,
        }
    }

    pub fn parts_mut(&mut self) -> Option<&mut Vec<Part>> {
        match &mut self.content {
            Content::Parts(parts) => Some(parts),
            Content::Text(_) => None,
        }
    }
}

/// Baca nilai providerOptions.<namespace>.<key> dari Map mentah.
pub fn provider_option<'a>(
    options: Option<&'a Map<String, Value>>,
    namespace: &str,
    key: &str,
) -> Option<&'a Value> {
    options?.get(namespace)?.get(key)
}
