//! Route handlers — core session CRUD + config + provider + health.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{json, Value};

use crate::AppState;
use oc_session::model::SessionRow;

type AppResult<T> = Result<Json<T>, StatusCode>;

pub async fn health() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

pub async fn list_sessions(State(store): State<AppState>) -> AppResult<Vec<SessionRow>> {
    store
        .list_sessions()
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn create_session(
    State(store): State<AppState>,
    body: Option<Json<Value>>,
) -> AppResult<SessionRow> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let id = format!("ses_{now}");
    let body_map = body.map(|Json(v)| v).unwrap_or_else(|| json!({}));
    let title = body_map["title"]
        .as_str()
        .unwrap_or("New session")
        .to_string();
    let directory = body_map["directory"].as_str().unwrap_or(".").to_string();
    let project_id = body_map["projectID"]
        .as_str()
        .unwrap_or("default")
        .to_string();
    let slug_suffix = &id[id.len().saturating_sub(8)..];
    let session = SessionRow {
        id: id.clone(),
        title,
        directory,
        version: "1.18.21".into(),
        slug: format!("session-{slug_suffix}"),
        project_id,
        time_created: now,
        time_updated: now,
        ..Default::default()
    };
    store
        .upsert_session(&session)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(session))
}

pub async fn get_session(
    State(store): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionRow>, StatusCode> {
    store
        .get_session(&session_id)
        .ok()
        .flatten()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn remove_session(
    State(store): State<AppState>,
    Path(session_id): Path<String>,
) -> StatusCode {
    match store.remove_session(&session_id) {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn list_messages(
    State(store): State<AppState>,
    Path(session_id): Path<String>,
) -> AppResult<Vec<oc_session::model::WithParts>> {
    store
        .list_messages(&session_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn send_message(
    State(store): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<Value>,
) -> AppResult<Value> {
    let msg_id = format!(
        "msg_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let message = oc_session::model::UserOrAssistant::User(oc_session::model::UserMessage {
        id: msg_id.clone(),
        session_id: session_id.clone(),
        time: oc_session::model::TimeCreated { created: now },
        format: None,
        summary: None,
        agent: body["agent"].as_str().unwrap_or("build").to_string(),
        model: oc_session::model::ModelRefJson {
            provider_id: body["providerID"]
                .as_str()
                .unwrap_or("anthropic")
                .to_string(),
            model_id: body["modelID"]
                .as_str()
                .unwrap_or("claude-sonnet-4")
                .to_string(),
        },
        system: None,
        tools: None,
    });
    store
        .append_message(&message)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "id": msg_id, "sessionID": session_id })))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({
        "default_agent": "build",
        "share": "disabled",
        "autoupdate": true
    }))
}

pub async fn list_providers() -> Json<Value> {
    Json(json!([]))
}

// --- Sprint 12b: additional routes ---

pub async fn update_session(
    State(store): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<SessionRow>, StatusCode> {
    let mut session = store
        .get_session(&session_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(title) = body["title"].as_str() {
        session.title = title.to_string();
    }
    if let Some(dir) = body["directory"].as_str() {
        session.directory = dir.to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    session.time_updated = now;

    store
        .upsert_session(&session)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(session))
}

pub async fn list_parts(
    State(store): State<AppState>,
    Path((session_id, message_id)): Path<(String, String)>,
) -> AppResult<Vec<oc_session::model::Part>> {
    store
        .list_parts(&session_id, &message_id)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_agent_list() -> Json<Value> {
    Json(json!([
        { "name": "build", "description": "Primary coding agent", "mode": "primary" },
        { "name": "plan", "description": "Planning agent", "mode": "primary" },
        { "name": "general", "description": "General purpose subagent", "mode": "subagent" },
        { "name": "explore", "description": "Codebase exploration agent", "mode": "subagent" },
        { "name": "compaction", "description": "Context compaction agent", "mode": "all" },
        { "name": "title", "description": "Session title generator", "mode": "all" },
        { "name": "summary", "description": "Session summary generator", "mode": "all" }
    ]))
}

pub async fn get_tool_list() -> Json<Value> {
    Json(json!([
        { "name": "read", "description": "Read file contents" },
        { "name": "write", "description": "Write file contents" },
        { "name": "edit", "description": "Edit file with targeted replacements" },
        { "name": "glob", "description": "Find files by pattern" },
        { "name": "grep", "description": "Search file contents" },
        { "name": "bash", "description": "Execute shell command" },
        { "name": "webfetch", "description": "Fetch web content" },
        { "name": "websearch", "description": "Search the web" },
        { "name": "task", "description": "Launch subagent" },
        { "name": "todowrite", "description": "Manage task list" },
        { "name": "apply_patch", "description": "Apply codex-style patches" }
    ]))
}

pub async fn get_model_list() -> Json<Value> {
    Json(json!([
        { "providerID": "anthropic", "modelID": "claude-sonnet-4", "name": "Claude Sonnet 4" },
        { "providerID": "anthropic", "modelID": "claude-opus-4", "name": "Claude Opus 4" },
        { "providerID": "openai", "modelID": "gpt-5", "name": "GPT-5" },
        { "providerID": "openai", "modelID": "gpt-4.1", "name": "GPT-4.1" },
        { "providerID": "google", "modelID": "gemini-2.5-pro", "name": "Gemini 2.5 Pro" }
    ]))
}
