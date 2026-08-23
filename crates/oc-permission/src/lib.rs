//! Ported from: packages/opencode/src/permission/index.ts
//! dan contracts dari packages/schema/src/v1/permission.ts +
//! packages/core/src/v1/permission.ts (error classes).

pub mod arity;
pub mod wildcard;

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use serde::Serialize;
use serde_json::Value;

// --- PermissionV1 contracts (schema/v1/permission.ts) ---

/// Ported from: schema/v1/permission.ts:8-14 (ID, prefix "per_")
pub type Id = String;

fn ascending_counter() -> &'static Mutex<u64> {
    static COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
    COUNTER.get_or_init(|| Mutex::new(0))
}

/// Padanan `PermissionV1.ID.ascending()` — token monotonik
/// (approx ULID TS; validasi hanya isStartsWith("per")).
pub fn id_ascending() -> Id {
    let mut guard = ascending_counter().lock().unwrap();
    *guard += 1;
    let counter = *guard;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("per_{millis:08x}{counter:04x}")
}

/// Ported from: schema/v1/permission.ts:16-19 (Action)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Allow,
    Deny,
    Ask,
}

impl Action {
    fn as_str(self) -> &'static str {
        match self {
            Action::Allow => "allow",
            Action::Deny => "deny",
            Action::Ask => "ask",
        }
    }
}

/// Ported from: schema/v1/permission.ts:21-25 (Rule / Ruleset)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rule {
    pub permission: String,
    pub pattern: String,
    pub action: Action,
}

pub type Ruleset = Vec<Rule>;

/// Ported from: schema/v1/permission.ts:27-38 (Request)
#[derive(Debug, Clone, Serialize)]
pub struct Request {
    pub id: Id,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: oc_config::v1::OrderedMap<Value>,
    pub always: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolRef {
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(rename = "callID")]
    pub call_id: String,
}

/// Ported from: schema/v1/permission.ts:40-42 (Reply)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    Once,
    Always,
    Reject,
}

/// Ported from: schema/v1/permission.ts:48-51 (AskInput)
#[derive(Debug, Clone)]
pub struct AskInput {
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: oc_config::v1::OrderedMap<Value>,
    pub always: Vec<String>,
    pub tool: Option<ToolRef>,
    /// `id` opsional pada AskInput (berbeda dari Request yang wajib)
    pub id: Option<Id>,
    pub ruleset: Ruleset,
}

/// Ported from: schema/v1/permission.ts:53-56 (ReplyInput)
#[derive(Debug, Clone)]
pub struct ReplyInput {
    pub request_id: Id,
    pub reply: Reply,
    pub message: Option<String>,
}

// --- Errors (core/v1/permission.ts) ---

/// Ported from: core/v1/permission.ts:4-8 (RejectedError)
#[derive(Debug)]
pub struct RejectedError;

impl std::fmt::Display for RejectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The user rejected permission to use this specific tool call."
        )
    }
}

/// Ported from: core/v1/permission.ts:10-15 (CorrectedError)
#[derive(Debug)]
pub struct CorrectedError {
    pub feedback: String,
}

impl std::fmt::Display for CorrectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The user rejected permission to use this specific tool call with the following feedback: {}",
            self.feedback
        )
    }
}

/// Ported from: core/v1/permission.ts:17-22 (DeniedError)
#[derive(Debug)]
pub struct DeniedError {
    pub ruleset: Value,
}

impl DeniedError {
    pub fn message(&self) -> String {
        format!(
            "The user has specified a rule which prevents you from using this specific tool call. Here are some of the relevant rules {}",
            serde_json::to_string(&self.ruleset).unwrap_or_else(|_| "null".to_string())
        )
    }
}

/// Ported from: core/v1/permission.ts:26-28 (NotFoundError)
#[derive(Debug)]
pub struct NotFoundError {
    pub request_id: Id,
}

/// Ported from: core/v1/permission.ts:30 (`type Error`)
#[derive(Debug)]
pub enum Error {
    Denied(DeniedError),
    Rejected(RejectedError),
    Corrected(CorrectedError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Denied(error) => write!(f, "{}", error.message()),
            Error::Rejected(error) => write!(f, "{error}"),
            Error::Corrected(error) => write!(f, "{error}"),
        }
    }
}

// --- evaluate (index.ts:28-38) ---

/// Ported from: permission/index.ts:28-38 (evaluate)
/// Flat semua ruleset lalu findLast yang cocok; fallback ask.
pub fn evaluate(permission: &str, pattern: &str, rulesets: &[Ruleset]) -> Rule {
    rulesets
        .iter()
        .flatten()
        .rev()
        .find(|rule| {
            wildcard::r#match(permission, &rule.permission)
                && wildcard::r#match(pattern, &rule.pattern)
        })
        .cloned()
        .unwrap_or_else(|| Rule {
            action: Action::Ask,
            permission: permission.to_string(),
            pattern: "*".to_string(),
        })
}

// --- expand + fromConfig + merge + disabled/visibleTools ---

/// Ported from: index.ts:178-184 (expand, privat)
fn expand_pattern(pattern: &str) -> String {
    let home = home_dir();
    if let Some(rest) = pattern.strip_prefix("~/") {
        return format!("{home}/{rest}");
    }
    if pattern == "~" {
        return home;
    }
    if let Some(rest) = pattern.strip_prefix("$HOME/") {
        return format!("{home}/{rest}");
    }
    if let Some(rest) = pattern.strip_prefix("$HOME") {
        return format!("{home}{rest}");
    }
    pattern.to_string()
}

fn home_dir() -> String {
    #[cfg(windows)]
    let value = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let value = std::env::var_os("HOME");
    value
        .map(|v| v.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn rule_action_from_str(value: &str) -> Option<Action> {
    match value {
        "allow" => Some(Action::Allow),
        "deny" => Some(Action::Deny),
        "ask" => Some(Action::Ask),
        _ => None,
    }
}

/// Ported from: index.ts:186-198 (fromConfig) — versi Value menjaga urutan
/// kunci persis seperti Object.entries.
pub fn from_config_value(permission: &Value) -> Vec<Rule> {
    let mut ruleset = Vec::new();
    let Some(map) = permission.as_object() else {
        return ruleset;
    };
    for (key, value) in map {
        match value {
            Value::String(action) => {
                if let Some(action) = rule_action_from_str(action) {
                    ruleset.push(Rule {
                        permission: key.clone(),
                        action,
                        pattern: "*".to_string(),
                    });
                }
            }
            Value::Object(entries) => {
                for (pattern, action) in entries {
                    if let Some(action) = action.as_str().and_then(rule_action_from_str) {
                        ruleset.push(Rule {
                            permission: key.clone(),
                            pattern: expand_pattern(pattern),
                            action,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    ruleset
}

/// Convenience wrapper menerima Info typed dari oc-config.
/// CATATAN urutan: urutan kunci mengikuti serialisasi typed struct (field
/// dikenal lebih dulu), bisa beda dari JSON asli bila tercampur.
pub fn from_config_info(info: &oc_config::v1::permission::PermissionInfo) -> Vec<Rule> {
    let value = serde_json::to_value(info).unwrap_or(Value::Null);
    from_config_value(&value)
}

/// Ported from: index.ts:200-202 (merge)
pub fn merge_rulesets(rulesets: &[Ruleset]) -> Ruleset {
    rulesets.iter().flatten().cloned().collect()
}

const EDITS: &[&str] = &["edit", "write", "apply_patch"];
const READS: &[&str] = &[
    "list_mcp_resources",
    "list_mcp_resource_templates",
    "read_mcp_resource",
];

/// Ported from: index.ts:204-214 (disabled)
pub fn disabled_tools(tools: &[String], ruleset: &Ruleset) -> std::collections::HashSet<String> {
    tools
        .iter()
        .filter(|tool| {
            let permission = if EDITS.contains(&tool.as_str()) {
                "edit"
            } else if READS.contains(&tool.as_str()) {
                "read"
            } else {
                tool.as_str()
            };
            let rule = ruleset
                .iter()
                .rev()
                .find(|rule| wildcard::r#match(permission, &rule.permission));
            match rule {
                Some(rule) => rule.pattern == "*" && rule.action == Action::Deny,
                None => false,
            }
        })
        .cloned()
        .collect()
}

/// Ported from: index.ts:216-219 (visibleTools)
pub fn visible_tools<T>(tools: &[(String, T)], ruleset: &Ruleset) -> Vec<(String, T)>
where
    T: Clone,
{
    let names: Vec<String> = tools.iter().map(|(name, _)| name.clone()).collect();
    let hidden = disabled_tools(&names, ruleset);
    tools
        .iter()
        .filter(|(name, _)| !hidden.contains(name))
        .cloned()
        .collect()
}

// --- Service (index.ts:42-176) ---

enum Outcome {
    Resolved,
    Rejected,
    Corrected(String),
}

struct PendingEntry {
    info: Request,
    monitor: Arc<(Mutex<Option<Outcome>>, Condvar)>,
}

#[derive(Default)]
struct State {
    pending: HashMap<Id, PendingEntry>,
    approved: Vec<Rule>,
}

/// Pengganti EventV2Bridge.publish — dependency injection.
pub trait EventSink: Send + Sync {
    fn asked(&self, _info: &Request) {}
    fn replied(&self, _session_id: &str, _request_id: &str, _reply: Reply) {}
}

pub fn emit_asked(sink: &(dyn EventSink + 'static), info: &Request) {
    sink.asked(info)
}

pub fn emit_replied(
    sink: &(dyn EventSink + 'static),
    session_id: &str,
    request_id: &str,
    reply: Reply,
) {
    sink.replied(session_id, request_id, reply)
}

pub struct NoopSink;

impl EventSink for NoopSink {}

/// Ported from: index.ts:12-16 + 40-50 (Interface + Service)
pub struct PermissionService {
    state: Mutex<State>,
    sink: Box<dyn EventSink>,
}

impl Default for PermissionService {
    fn default() -> Self {
        Self::new(Box::new(NoopSink))
    }
}

impl PermissionService {
    pub fn new(sink: Box<dyn EventSink>) -> Self {
        PermissionService {
            state: Mutex::new(State::default()),
            sink,
        }
    }

    /// Ported from: index.ts:67-107 (ask) — blokir sampai reply masuk.
    pub fn ask(&self, input: AskInput) -> Result<(), Error> {
        // const { approved } = ... — snapshot sekali sebelum loop (sesuai TS)
        let approved_snapshot: Ruleset = self.state.lock().unwrap().approved.clone();
        let mut needs_ask = false;
        for pattern in &input.patterns {
            let rule = evaluate(
                &input.permission,
                pattern,
                &[input.ruleset.clone(), approved_snapshot.clone()],
            );
            tracing::info!(
                permission = %input.permission,
                pattern = %pattern,
                action = rule.action.as_str(),
                "evaluated"
            );
            match rule.action {
                Action::Deny => {
                    let filtered: Vec<&Rule> = input
                        .ruleset
                        .iter()
                        .filter(|rule| wildcard::r#match(&input.permission, &rule.permission))
                        .collect();
                    let ruleset = serde_json::to_value(filtered).unwrap_or(Value::Null);
                    return Err(Error::Denied(DeniedError { ruleset }));
                }
                Action::Allow => continue,
                Action::Ask => needs_ask = true,
            }
        }
        if !needs_ask {
            return Ok(());
        }

        let id = input.id.clone().unwrap_or_else(id_ascending);
        let info = Request {
            id: id.clone(),
            session_id: input.session_id.clone(),
            permission: input.permission.clone(),
            patterns: input.patterns.clone(),
            metadata: input.metadata.clone(),
            always: input.always.clone(),
            tool: input.tool.clone(),
        };
        tracing::info!(id = %id, permission = %info.permission, "asking");

        let monitor = Arc::new((Mutex::new(None), Condvar::new()));
        {
            let mut state = self.state.lock().unwrap();
            state.pending.insert(
                id.clone(),
                PendingEntry {
                    info: info.clone(),
                    monitor: monitor.clone(),
                },
            );
        }

        {
            // publish Asked tanpa memegang lock state
            emit_asked(&*self.sink, &info);
        }

        // Deferred.await
        let (mutex, condvar) = &*monitor;
        let mut outcome_guard = mutex.lock().unwrap();
        while outcome_guard.is_none() {
            outcome_guard = condvar.wait(outcome_guard).unwrap();
        }
        match outcome_guard.take().unwrap() {
            Outcome::Resolved => Ok(()),
            Outcome::Rejected => Err(Error::Rejected(RejectedError)),
            Outcome::Corrected(feedback) => Err(Error::Corrected(CorrectedError { feedback })),
        }
    }

    /// Ported from: index.ts:109-167 (reply)
    pub fn reply(&self, input: ReplyInput) -> Result<(), NotFoundError> {
        let existing = {
            let mut state = self.state.lock().unwrap();
            state.pending.remove(&input.request_id)
        };
        let Some(entry) = existing else {
            return Err(NotFoundError {
                request_id: input.request_id,
            });
        };

        emit_replied(
            &*self.sink,
            &entry.info.session_id,
            &entry.info.id,
            input.reply,
        );

        if input.reply == Reply::Reject {
            let outcome = match input.message {
                Some(message) => Outcome::Corrected(message),
                None => Outcome::Rejected,
            };
            {
                let (mutex, condvar) = &*entry.monitor;
                mutex.lock().unwrap().replace(outcome);
                condvar.notify_all();
            }
            // cascade reject untuk pending lain pada session sama
            let cascade: Vec<PendingEntry> = {
                let mut state = self.state.lock().unwrap();
                let ids: Vec<Id> = state
                    .pending
                    .iter()
                    .filter(|(_, item)| item.info.session_id == entry.info.session_id)
                    .map(|(id, _)| id.clone())
                    .collect();
                ids.into_iter()
                    .filter_map(|id| state.pending.remove(&id))
                    .collect()
            };
            for item in cascade {
                self.sink
                    .replied(&item.info.session_id, &item.info.id, Reply::Reject);
                let (mutex, condvar) = &*item.monitor;
                mutex.lock().unwrap().replace(Outcome::Rejected);
                condvar.notify_all();
            }
            return Ok(());
        }

        {
            let (mutex, condvar) = &*entry.monitor;
            mutex.lock().unwrap().replace(Outcome::Resolved);
            condvar.notify_all();
        }
        if input.reply == Reply::Once {
            return Ok(());
        }

        // always → push approved rules
        let auto_resolved: Vec<PendingEntry> = {
            let mut state = self.state.lock().unwrap();
            for pattern in &entry.info.always {
                state.approved.push(Rule {
                    permission: entry.info.permission.clone(),
                    pattern: pattern.clone(),
                    action: Action::Allow,
                });
            }
            // cascade: pending lain pada session sama yang sekarang fully allowed
            let approved_snapshot: Ruleset = state.approved.clone();
            let ids: Vec<Id> = state
                .pending
                .iter()
                .filter(|(_, item)| item.info.session_id == entry.info.session_id)
                .filter(|(_, item)| {
                    item.info.patterns.iter().all(|pattern| {
                        evaluate(
                            &item.info.permission,
                            pattern,
                            std::slice::from_ref(&approved_snapshot),
                        )
                        .action
                            == Action::Allow
                    })
                })
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter()
                .filter_map(|id| state.pending.remove(&id))
                .collect()
        };
        for item in auto_resolved {
            emit_replied(
                &*self.sink,
                &item.info.session_id,
                &item.info.id,
                Reply::Always,
            );
            let (mutex, condvar) = &*item.monitor;
            mutex.lock().unwrap().replace(Outcome::Resolved);
            condvar.notify_all();
        }
        Ok(())
    }

    /// Ported from: index.ts:169-172 (list)
    pub fn list(&self) -> Vec<Request> {
        let state = self.state.lock().unwrap();
        state
            .pending
            .values()
            .map(|item| item.info.clone())
            .collect()
    }
}
