mod common;

use axum::body::Body;
use axum::http::{header::CONTENT_TYPE, Request, StatusCode};
use common::test_state;
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::repository::devices::{create_device, find_device_by_rustdesk_id};
use serde_json::json;
use tower::ServiceExt;

async fn post(app: axum::Router, uri: &str, value: serde_json::Value) -> (StatusCode, Vec<u8>) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, body)
}

#[tokio::test]
async fn heartbeat_acknowledges_without_enrolling_or_controlling_sessions() {
    let state = test_state().await;
    let app = build_router(state.clone());
    let (status, body) = post(
        app,
        "/api/heartbeat",
        json!({
            "id":"123456",
            "uuid":"client-uuid",
            "ver":1004080,
            "conns":[1, 2],
            "modified_at":0
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"{}");
    assert!(find_device_by_rustdesk_id(&state.db, "123456")
        .await
        .expect("lookup")
        .is_none());
}

#[tokio::test]
async fn sysinfo_accepts_platform_fields_without_mutating_inventory() {
    let state = test_state().await;
    create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("654321".to_string()),
            alias: "original alias".to_string(),
            hostname: Some("original-host".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let app = build_router(state.clone());
    let (status, body) = post(
        app,
        "/api/sysinfo",
        json!({
            "id":"654321",
            "uuid":"client-uuid",
            "version":"1.4.9",
            "hostname":"untrusted-host",
            "os":"linux",
            "cpu":{"model":"example","cores":16},
            "memory":32768,
            "preset-address-book-password":"must-not-persist"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"SYSINFO_UPDATED");
    let unchanged = find_device_by_rustdesk_id(&state.db, "654321")
        .await
        .expect("lookup")
        .expect("device");
    assert_eq!(unchanged.alias, "original alias");
    assert_eq!(unchanged.hostname.as_deref(), Some("original-host"));
    assert!(unchanged.last_checkin_at.is_none());
}

#[tokio::test]
async fn compatibility_boundary_rejects_unknown_heartbeat_and_unbounded_sysinfo() {
    let state = test_state().await;
    let app = build_router(state);
    let (status, _) = post(
        app.clone(),
        "/api/heartbeat",
        json!({
            "id":"1",
            "uuid":"uuid",
            "ver":1,
            "modified_at":0,
            "unexpected":true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _) = post(
        app,
        "/api/sysinfo",
        json!({
            "id":"1",
            "uuid":"uuid",
            "version":"1.4.9",
            "hostname":"x".repeat(1025)
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn official_1_4_9_connection_audit_shapes_are_acknowledged_without_html() {
    let app = build_router(test_state().await);
    let payloads = [
        json!({
            "id":"123456",
            "uuid":"encoded-client-uuid",
            "conn_id":1,
            "session_id":7,
            "ip":"192.0.2.10",
            "action":"new"
        }),
        json!({
            "id":"123456",
            "uuid":"encoded-client-uuid",
            "conn_id":1,
            "session_id":7,
            "peer":["654321", "Remote User"],
            "type":0
        }),
        json!({
            "id":"123456",
            "session_id":7,
            "note":"maintenance"
        }),
        json!({
            "id":"123456",
            "uuid":"encoded-client-uuid",
            "conn_id":1,
            "session_id":7,
            "action":"close"
        }),
    ];
    for payload in payloads {
        let (status, body) = post(app.clone(), "/api/audit/conn", payload).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_empty());
    }
}

#[tokio::test]
async fn connection_audit_malformed_body_returns_json_error() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/audit/conn")
                .header("content-type", "application/json")
                .body(Body::from("not-json"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    assert_eq!(body, r#"{"error":"Invalid request"}"#);
}
