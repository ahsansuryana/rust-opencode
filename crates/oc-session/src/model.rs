//! Ported dari packages/schema/src/v1/session.ts dan packages/core/src/session/sql.ts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- BaseIds (partBase: id, sessionID, messageID) ---

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaseIds {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "messageID")]
    pub message_id: String,
}

// --- Time helpers ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCreated {
    pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeStart {
    pub start: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCompleted {
    pub start: u64,
    pub end: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWithCompletion {
    pub created: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<u64>,
}

// --- Tokens ---

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub reasoning: f64,
    pub cache: CacheReadWrite,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheReadWrite {
    pub read: f64,
    pub write: f64,
}

// --- Model ref ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRefJson {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

// --- FilePartSource ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilePartSource {
    File {
        path: String,
        text: SourceText,
    },
    Symbol {
        path: String,
        text: SourceText,
        range: Range,
        name: String,
        kind: u64,
    },
    Resource {
        client_name: String,
        uri: String,
        text: SourceText,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceText {
    pub value: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

// --- ToolState ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToolState {
    Pending {
        input: BTreeMap<String, Value>,
        raw: String,
    },
    Running {
        input: BTreeMap<String, Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, Value>>,
        time: TimeStart,
    },
    Completed {
        input: BTreeMap<String, Value>,
        output: String,
        title: String,
        metadata: BTreeMap<String, Value>,
        time: TimeCompleted,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attachments: Option<Vec<Value>>,
    },
    Error {
        input: BTreeMap<String, Value>,
        error: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, Value>>,
        time: TimeCompleted,
    },
}

// --- Part union (12 varian) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    Snapshot {
        #[serde(flatten)]
        ids: BaseIds,
        snapshot: String,
    },
    Patch {
        #[serde(flatten)]
        ids: BaseIds,
        hash: String,
        files: Vec<String>,
    },
    Text {
        #[serde(flatten)]
        ids: BaseIds,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        synthetic: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ignored: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        time: Option<TimeRange>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, Value>>,
    },
    Reasoning {
        #[serde(flatten)]
        ids: BaseIds,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, Value>>,
        time: TimeRange,
    },
    File {
        #[serde(flatten)]
        ids: BaseIds,
        mime: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<FilePartSource>,
    },
    Agent {
        #[serde(flatten)]
        ids: BaseIds,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<Value>,
    },
    Compaction {
        #[serde(flatten)]
        ids: BaseIds,
        auto: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        overflow: Option<bool>,
        #[serde(
            rename = "tail_start_id",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        tail_start_id: Option<String>,
    },
    Subtask {
        #[serde(flatten)]
        ids: BaseIds,
        prompt: String,
        description: String,
        agent: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<ModelRefJson>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
    },
    Retry {
        #[serde(flatten)]
        ids: BaseIds,
        attempt: u64,
        error: Value,
        time: TimeCreated,
    },
    StepStart {
        #[serde(flatten)]
        ids: BaseIds,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snapshot: Option<String>,
    },
    StepFinish {
        #[serde(flatten)]
        ids: BaseIds,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snapshot: Option<String>,
        cost: f64,
        tokens: TokenUsage,
    },
    Tool {
        #[serde(flatten)]
        ids: BaseIds,
        #[serde(rename = "callID")]
        call_id: String,
        tool: String,
        state: ToolState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        metadata: Option<BTreeMap<String, Value>>,
    },
}

impl Part {
    pub fn base_ids(&self) -> &BaseIds {
        match self {
            Part::Snapshot { ids, .. }
            | Part::Patch { ids, .. }
            | Part::Text { ids, .. }
            | Part::Reasoning { ids, .. }
            | Part::File { ids, .. }
            | Part::Agent { ids, .. }
            | Part::Compaction { ids, .. }
            | Part::Subtask { ids, .. }
            | Part::Retry { ids, .. }
            | Part::StepStart { ids, .. }
            | Part::StepFinish { ids, .. }
            | Part::Tool { ids, .. } => ids,
        }
    }
}

// --- Messages ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum UserOrAssistant {
    User(UserMessage),
    Assistant(AssistantMessage),
}

impl UserOrAssistant {
    pub fn session_id(&self) -> &str {
        match self {
            Self::User(m) => &m.session_id,
            Self::Assistant(m) => &m.session_id,
        }
    }
    pub fn id(&self) -> &str {
        match self {
            Self::User(m) => &m.id,
            Self::Assistant(m) => &m.id,
        }
    }
    pub fn created(&self) -> u64 {
        match self {
            Self::User(m) => m.time.created,
            Self::Assistant(m) => m.time.created,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub time: TimeCreated,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<Value>,
    pub agent: String,
    pub model: ModelRefJson,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<BTreeMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub time: TimeWithCompletion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
    #[serde(rename = "parentID")]
    pub parent_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub mode: String,
    pub agent: String,
    pub path: SessionPath,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<bool>,
    pub cost: f64,
    pub tokens: TokenUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPath {
    pub cwd: String,
    pub root: String,
}

/// WithParts — pesan + parts terkait.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithParts {
    #[serde(flatten)]
    pub info: UserOrAssistant,
    #[serde(default)]
    pub parts: Vec<Part>,
}

// --- SessionRow (SessionTable row shape) ---

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    #[serde(rename = "project_id", default)]
    pub project_id: String,
    #[serde(
        rename = "workspace_id",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub workspace_id: Option<String>,
    #[serde(rename = "parent_id", default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub directory: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(rename = "share_url", default, skip_serializing_if = "Option::is_none")]
    pub share_url: Option<String>,
    #[serde(
        rename = "summary_additions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub summary_additions: Option<i64>,
    #[serde(
        rename = "summary_deletions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub summary_deletions: Option<i64>,
    #[serde(
        rename = "summary_files",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub summary_files: Option<i64>,
    #[serde(
        rename = "summary_diffs",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub summary_diffs: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    pub cost: f64,
    #[serde(rename = "tokens_input", default)]
    pub tokens_input: i64,
    #[serde(rename = "tokens_output", default)]
    pub tokens_output: i64,
    #[serde(rename = "tokens_reasoning", default)]
    pub tokens_reasoning: i64,
    #[serde(rename = "tokens_cache_read", default)]
    pub tokens_cache_read: i64,
    #[serde(rename = "tokens_cache_write", default)]
    pub tokens_cache_write: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revert: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Value>,
    #[serde(rename = "time_created", default)]
    pub time_created: u64,
    #[serde(rename = "time_updated", default)]
    pub time_updated: u64,
    #[serde(
        rename = "time_compacting",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub time_compacting: Option<u64>,
    #[serde(
        rename = "time_archived",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub time_archived: Option<u64>,
}

/// Alias untuk kompatibilitas.
pub type Session = SessionRow;

/// Ported dari message-v2.ts:latest() — find latest user/assistant/finished messages
/// and pending tasks (compaction/subtask parts).
#[derive(Debug, Clone, Default)]
pub struct LatestResult {
    pub user: Option<WithParts>,
    pub assistant: Option<WithParts>,
    pub finished: Option<WithParts>,
    pub tasks: Vec<Part>,
}

pub fn latest(msgs: &[WithParts]) -> LatestResult {
    let mut result = LatestResult::default();

    // Find latest user message
    for m in msgs.iter().rev() {
        if matches!(m.info, UserOrAssistant::User(_)) {
            result.user = Some(m.clone());
            break;
        }
    }

    // Find latest assistant/finished message and extract pending tasks
    let mut finished: Option<WithParts> = None;
    for m in msgs.iter().rev() {
        if let UserOrAssistant::Assistant(a) = &m.info {
            if a.time.completed.is_some() && finished.is_none() {
                finished = Some(m.clone());
            }
            if result.assistant.is_none() {
                result.assistant = Some(m.clone());
            }
        }
    }
    result.finished = finished;

    // Extract pending subtask/compaction parts from messages that are after finished
    if let Some(ref fin) = result.finished {
        for m in msgs {
            // skip messages before finished
            if m.info.created() <= fin.info.created() {
                continue;
            }
            for p in &m.parts {
                if matches!(p, Part::Subtask { .. } | Part::Compaction { .. }) {
                    result.tasks.push(p.clone());
                }
            }
        }
    } else {
        // no finished message → all tasks are pending
        for m in msgs {
            for p in &m.parts {
                if matches!(p, Part::Subtask { .. } | Part::Compaction { .. }) {
                    result.tasks.push(p.clone());
                }
            }
        }
    }

    result
}
