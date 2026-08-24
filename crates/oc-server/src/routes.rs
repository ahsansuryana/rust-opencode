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
