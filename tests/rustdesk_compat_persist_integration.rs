mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::test_state;
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::repository::audit_events::list_audit_events;
use opendesk::repository::devices::{create_device, find_device_by_rustdesk_id};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn post(app: axum::Router, uri: &str, value: serde_json::Value) -> (StatusCode, Vec<u8>) {
    send(app, "POST", uri, value).await
}

async fn put(app: axum::Router, uri: &str, value: serde_json::Value) -> (StatusCode, Vec<u8>) {
    send(app, "PUT", uri, value).await
}

async fn send(
    app: axum::Router,
    method: &str,
    uri: &str,
    value: serde_json::Value,
) -> (StatusCode, Vec<u8>) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
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

fn heartbeat_body(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "uuid": "client-uuid",
        "ver": 1004080,
        "conns": [1, 2],
        "modified_at": 0
    })
}

#[tokio::test]
async fn heartbeat_updates_last_checkin_for_existing_device() {
    let state = test_state().await;
    create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("777001".to_string()),
            alias: "presence device".to_string(),
            hostname: Some("presence-host".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let app = build_router(state.clone());
    let (status, body) = post(app, "/api/heartbeat", heartbeat_body("777001")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"{}");
    let device = find_device_by_rustdesk_id(&state.db, "777001")
        .await
        .expect("lookup")
        .expect("device");
    assert!(device.last_checkin_at.is_some());
    assert_eq!(device.alias, "presence device");
    assert_eq!(device.hostname.as_deref(), Some("presence-host"));
}

#[tokio::test]
async fn sysinfo_unknown_id_does_not_create_device() {
    let state = test_state().await;
    let app = build_router(state.clone());
    let (status, body) = post(
        app,
        "/api/sysinfo",
        json!({
            "id":"unknown-sys",
            "uuid":"client-uuid",
            "version":"1.4.9",
            "hostname":"ghost-host",
            "os":"linux"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"SYSINFO_UPDATED");
    assert!(find_device_by_rustdesk_id(&state.db, "unknown-sys")
        .await
        .expect("lookup")
        .is_none());
}

fn detail_json(event: &opendesk::repository::audit_events::AuditEventRow) -> Value {
    serde_json::from_str(event.detail_json.as_deref().expect("detail")).expect("json")
}

#[tokio::test]
async fn connection_audit_persists_redacted_event_for_existing_device() {
    let state = test_state().await;
    let device = create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("123456".to_string()),
            alias: "audited device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let app = build_router(state.clone());
    let (status, body) = post(
        app,
        "/api/audit/conn",
        json!({
            "id":"123456",
            "uuid":"encoded-client-uuid",
            "conn_id":1,
            "session_id":7,
            "ip":"192.0.2.10",
            "action":"new"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
    let events = list_audit_events(&state.db, 10).await.expect("audit");
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.action, "client_connection_open");
    assert_eq!(event.object_type, "device");
    assert_eq!(event.object_uuid, Some(device.device_uuid));
    assert_eq!(event.outcome, "accepted");
    assert_eq!(event.source, "official_client");
    let detail = detail_json(event);
    assert_eq!(detail["ip"], "192.0.2.10");
    assert_eq!(detail["rustdesk_id"], "123456");
    assert!(event
        .detail_json
        .as_deref()
        .expect("detail")
        .contains("192.0.2.10"));
}

#[tokio::test]
async fn audit_note_and_file_redact_secrets_without_storing_blobs() {
    let state = test_state().await;
    create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("123456".to_string()),
            alias: "audited device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let app = build_router(state.clone());
    let (status, _) = post(
        app.clone(),
        "/api/audit",
        json!({
            "id":"123456",
            "note":"maintenance",
            "password":"must-not-persist"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = put(
        app.clone(),
        "/api/audit",
        json!({
            "guid":"audit-guid-1",
            "note":"closed"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post(
        app.clone(),
        "/api/audit/file",
        json!({
            "id":"123456",
            "uuid":"encoded-client-uuid",
            "peer_id":"654321",
            "type":0,
            "path":"/tmp/example.txt",
            "is_file":true,
            "info": {"password":"must-not-persist","ip":"192.0.2.10"}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let events = list_audit_events(&state.db, 10).await.expect("audit");
    let note = events
        .iter()
        .find(|event| event.action == "client_audit_note" && event.object_type == "device")
        .expect("note");
    let note_detail = detail_json(note);
    assert_eq!(note_detail["password"], "[redacted]");
    assert_eq!(note_detail["note"], "maintenance");
    assert_ne!(note_detail["password"], "must-not-persist");

    let file = events
        .iter()
        .find(|event| event.action == "client_file_transfer")
        .expect("file");
    assert_eq!(file.object_type, "device");
    assert_eq!(file.source, "official_client");
    let file_detail = detail_json(file);
    assert_eq!(file_detail["info"]["password"], "[redacted]");
    assert_eq!(file_detail["info"]["ip"], "192.0.2.10");
    assert_eq!(file_detail["path"], "/tmp/example.txt");
}

#[tokio::test]
async fn record_upload_acknowledges_without_storing_and_rejects_invalid_type() {
    let state = test_state().await;
    let app = build_router(state.clone());
    let (status, body) = post(
        app.clone(),
        "/api/record?type=new&file=session.mp4",
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let response: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(response["stored"], false);
    assert!(response.get("error").is_none());
    let events = list_audit_events(&state.db, 10).await.expect("audit");
    assert_eq!(events[0].action, "client_session_record");
    assert_eq!(events[0].source, "official_client");
    let detail = detail_json(&events[0]);
    assert_eq!(detail["stored"], false);
    assert_eq!(detail["file"], "session.mp4");

    let (status, _) = post(app, "/api/record?type=video&file=session.mp4", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn oversized_and_invalid_liveness_bodies_still_rejected() {
    let app = build_router(test_state().await);
    let (status, _) = post(
        app.clone(),
        "/api/audit/conn",
        json!({
            "id":"123456",
            "session_id":7,
            "note":"x".repeat(1025)
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = post(
        app.clone(),
        "/api/audit",
        json!({
            "id":"123456",
            "note":"x".repeat(1025)
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = post(
        app.clone(),
        "/api/audit/file",
        json!({
            "id":"123456",
            "path":"x".repeat(1025)
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/conn/active?session_id=7&conn_type=0")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn active_connection_audit_returns_json_string_guid() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/conn/active?id=123456&session_id=7&conn_type=0")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let guid: String = serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json string");
    assert_eq!(guid.len(), 36);
}
