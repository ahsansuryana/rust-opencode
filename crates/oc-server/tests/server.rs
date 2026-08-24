use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn app() -> axum::Router {
    let root = std::env::temp_dir().join(format!("oc-server-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::env::set_var("HOME", root.to_str().unwrap());
    std::env::set_var("USERPROFILE", root.to_str().unwrap());
    for key in [
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
    ] {
        std::env::remove_var(key);
    }
    oc_global::reset_for_tests();
    let store = oc_session::store::SessionStore::new().unwrap();
    oc_server::router(store)
}

#[tokio::test]
async fn health_endpoint() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn session_crud_flow() {
    let app = app();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title": "Test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let session_id = body["id"].as_str().unwrap().to_string();

    // GET /session/{id}
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/session/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // GET /session/{id}/message
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/session/{session_id}/message"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // DELETE /session/{id}
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/session/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // verify deleted
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/session/{session_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
