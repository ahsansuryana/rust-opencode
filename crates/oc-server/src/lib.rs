//! HTTP server untuk rust-opencode — subset core endpoints dari openapi.json.
//! Full 150+ endpoint terdaftar di ENDPOINT_CHECKLIST.md; sprint ini
//! mengimplementasikan yang paling esensial.

pub mod routes;

use axum::response::{Html, IntoResponse};
use axum::routing::{delete, get, post};
use axum::Router;
use oc_session::store::SessionStore;
use std::sync::Arc;

pub type AppState = Arc<SessionStore>;

/// Embedded web UI — di-compile ke dalam binary.
const INDEX_HTML: &str = include_str!("../static/index.html");

async fn serve_ui() -> impl IntoResponse {
    Html(INDEX_HTML.to_string())
}

pub fn router(store: SessionStore) -> Router {
    let state: AppState = Arc::new(store);
    Router::new()
        // Web UI
        .route("/", get(serve_ui))
        .route("/ui", get(serve_ui))
        // API routes
        .route("/health", get(routes::health))
        .route("/session", get(routes::list_sessions))
        .route("/session", post(routes::create_session))
        .route("/session/{session_id}", get(routes::get_session))
        .route("/session/{session_id}", delete(routes::remove_session))
        .route("/session/{session_id}", post(routes::update_session))
        .route("/session/{session_id}/message", get(routes::list_messages))
        .route("/session/{session_id}/message", post(routes::send_message))
        .route(
            "/session/{session_id}/message/{message_id}/part",
            get(routes::list_parts),
        )
        .route("/config", get(routes::get_config))
        .route("/provider", get(routes::list_providers))
        .route("/agent", get(routes::get_agent_list))
        .route("/tool", get(routes::get_tool_list))
        .route("/model", get(routes::get_model_list))
        .with_state(state)
}
