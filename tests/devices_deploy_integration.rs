mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::test_state;
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::repository::devices::{create_device, find_device_by_rustdesk_id, list_devices};
use opendesk::repository::enrollment_tokens::{create_enrollment_token, revoke_enrollment_token};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn post_json(
    app: axum::Router,
    uri: &str,
    token: Option<&str>,
    body: Body,
) -> (StatusCode, Vec<u8>) {
    let mut request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, bytes)
}

fn deploy_body(id: &str) -> Body {
    Body::from(
        json!({
            "id": id,
            "uuid": "client-uuid",
            "pk": "cGs="
        })
        .to_string(),
    )
}

#[tokio::test]
async fn deploy_creates_and_updates_device_with_enrollment_token() {
    let state = test_state().await;
    let created = create_enrollment_token(&state.db, "deploy-token", None, None, None)
        .await
        .expect("create token");
    let db = state.db.clone();
    let app = build_router(state);
    let (status, body) = post_json(
        app.clone(),
        "/api/devices/deploy",
        Some(&created.token_value),
        Body::from(
            json!({
                "id": "998877001",
                "uuid": "client-uuid",
                "pk": "cGs=",
                "unexpected": true
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "OK");
    let first = find_device_by_rustdesk_id(&db, "998877001")
        .await
        .expect("lookup")
        .expect("device");
    assert_eq!(first.alias, "998877001");
    assert!(first.last_checkin_at.is_some());

    let (status, body) = post_json(
        app,
        "/api/devices/deploy",
        Some(&created.token_value),
        deploy_body("998877001"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "OK");
    let matches: Vec<_> = list_devices(&db)
        .await
        .expect("list")
        .into_iter()
        .filter(|device| device.rustdesk_id.as_deref() == Some("998877001"))
        .collect();
    assert_eq!(matches.len(), 1);
    assert!(matches[0].last_checkin_at.is_some());
}

#[tokio::test]
async fn deploy_missing_bearer_returns_invalid_input() {
    let state = test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let (status, body) =
        post_json(app, "/api/devices/deploy", None, deploy_body("998877002")).await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "INVALID_INPUT");
    assert!(find_device_by_rustdesk_id(&db, "998877002")
        .await
        .expect("lookup")
        .is_none());
}

#[tokio::test]
async fn deploy_revoked_token_returns_invalid_input() {
    let state = test_state().await;
    let created = create_enrollment_token(&state.db, "revoked-deploy", None, None, None)
        .await
        .expect("create token");
    revoke_enrollment_token(&state.db, created.record.enrollment_token_uuid)
        .await
        .expect("revoke");
    let db = state.db.clone();
    let app = build_router(state);
    let (status, body) = post_json(
        app,
        "/api/devices/deploy",
        Some(&created.token_value),
        deploy_body("998877003"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "INVALID_INPUT");
    assert!(find_device_by_rustdesk_id(&db, "998877003")
        .await
        .expect("lookup")
        .is_none());
}

#[tokio::test]
async fn deploy_empty_id_returns_invalid_input() {
    let state = test_state().await;
    let created = create_enrollment_token(&state.db, "empty-id-deploy", None, None, None)
        .await
        .expect("create token");
    let db = state.db.clone();
    let app = build_router(state);
    let (status, body) = post_json(
        app,
        "/api/devices/deploy",
        Some(&created.token_value),
        Body::from(
            json!({
                "id": "",
                "uuid": "client-uuid",
                "pk": "cGs="
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "INVALID_INPUT");
    assert!(list_devices(&db).await.expect("list").is_empty());
}

#[tokio::test]
async fn deploy_invalid_json_returns_invalid_input() {
    let state = test_state().await;
    let created = create_enrollment_token(&state.db, "json-deploy", None, None, None)
        .await
        .expect("create token");
    let app = build_router(state);
    let (status, body) = post_json(
        app,
        "/api/devices/deploy",
        Some(&created.token_value),
        Body::from("not-json"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "INVALID_INPUT");
}

#[tokio::test]
async fn deploy_unknown_token_redacts_audit_detail() {
    let state = test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let secret = "super-secret-deploy-enrollment-token-value";
    let (status, body) = post_json(
        app,
        "/api/devices/deploy",
        Some(secret),
        deploy_body("998877004"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(result["result"], "INVALID_INPUT");
    let events = opendesk::repository::audit_events::list_audit_events(&db, 20)
        .await
        .expect("audit");
    let failure = events
        .iter()
        .find(|event| event.action == "endpoint_checkin" && event.outcome == "failure")
        .expect("failure audit");
    let detail = failure.detail_json.as_deref().unwrap_or("");
    assert!(
        detail.contains("[redacted]"),
        "expected redacted detail, got {detail}"
    );
    assert!(
        !detail.contains(secret),
        "raw enrollment token must not appear in audit detail"
    );
}

#[tokio::test]
async fn cli_maps_alias_and_notes_and_ignores_address_book_password() {
    let state = test_state().await;
    let created = create_enrollment_token(&state.db, "cli-token", None, None, None)
        .await
        .expect("create token");
    create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("998877005".to_string()),
            alias: "original".to_string(),
            notes: Some("old-note".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("seed");
    let db = state.db.clone();
    let app = build_router(state);
    let password = "must-not-persist-address-book-password";
    let (status, body) = post_json(
        app,
        "/api/devices/cli",
        Some(&created.token_value),
        Body::from(
            json!({
                "id": "998877005",
                "uuid": "client-uuid",
                "device_name": "lab-pc",
                "note": "shelf-a",
                "address_book_password": password,
                "device_username": "ignored-user"
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
    let device = find_device_by_rustdesk_id(&db, "998877005")
        .await
        .expect("lookup")
        .expect("device");
    assert_eq!(device.alias, "lab-pc");
    assert_eq!(device.notes.as_deref(), Some("shelf-a"));
    assert_ne!(device.owner.as_deref(), Some("ignored-user"));
    assert!(device.last_checkin_at.is_some());
    let blob = format!(
        "{}{}{}",
        device.alias,
        device.notes.unwrap_or_default(),
        device.owner.unwrap_or_default()
    );
    assert!(!blob.contains(password));
}

#[tokio::test]
async fn cli_missing_bearer_returns_error_text() {
    let state = test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let (status, body) = post_json(
        app,
        "/api/devices/cli",
        None,
        Body::from(
            json!({
                "id": "998877006",
                "uuid": "client-uuid",
                "device_name": "lab-pc"
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.is_empty());
    assert!(find_device_by_rustdesk_id(&db, "998877006")
        .await
        .expect("lookup")
        .is_none());
}
