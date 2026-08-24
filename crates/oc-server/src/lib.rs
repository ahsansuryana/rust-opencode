//! HTTP server untuk rust-opencode — subset core endpoints dari openapi.json.
//! Full 150+ endpoint terdaftar di ENDPOINT_CHECKLIST.md; sprint ini
//! mengimplementasikan yang paling esensial.

pub mod routes;

use axum::routing::{delete, get, post};
use axum::Router;
use oc_session::store::SessionStore;
use std::sync::Arc;

pub type AppState = Arc<SessionStore>;

pub fn router(store: SessionStore) -> Router {
    let state: AppState = Arc::new(store);
    Router::new()
        .route("/health", get(routes::health))
        .route("/session", get(routes::list_sessions))
        .route("/session", post(routes::create_session))
        .route("/session/{session_id}", get(routes::get_session))
        .route("/session/{session_id}", delete(routes::remove_session))
        .route("/session/{session_id}/message", get(routes::list_messages))
        .route("/session/{session_id}/message", post(routes::send_message))
        .route("/config", get(routes::get_config))
        .route("/provider", get(routes::list_providers))
        .with_state(state)
}
