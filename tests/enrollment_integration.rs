mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use common::{login_and_get_session_cookie, test_state};
use opendesk::build_router;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn enrollment_checkin_creates_device_with_valid_token() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "integration-token",
        None,
        None,
        None,
    )
    .await
    .expect("create token");

    let db = state.db.clone();
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "998877665",
                        "hostname": "dev-host",
                        "os_family": "linux",
                        "architecture": "x86_64"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let device = opendesk::repository::devices::find_device_by_rustdesk_id(&db, "998877665")
        .await
        .expect("lookup")
        .expect("device");
    assert!(device.last_checkin_at.is_some());
}

#[tokio::test]
async fn enrollment_checkin_updates_existing_device_without_duplicate() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "duplicate-rustdesk-id",
        None,
        None,
        None,
    )
    .await
    .expect("create token");
    let app = build_router(state.clone());
    let checkin_body = |rustdesk_version: &str| {
        json!({
            "enrollment_token": created.token_value,
            "rustdesk_id": "445566778",
            "hostname": "ws-dup-01",
            "os_family": "linux",
            "architecture": "x86_64",
            "rustdesk_version": rustdesk_version
        })
        .to_string()
    };
    for rustdesk_version in ["1.4.7", "1.4.8"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/enrollments/check-in")
                    .header("content-type", "application/json")
                    .body(Body::from(checkin_body(rustdesk_version)))
                    .unwrap(),
            )
            .await
            .expect("checkin");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
    let devices = opendesk::repository::devices::list_devices(&state.db)
        .await
        .expect("list devices");
    let matches: Vec<_> = devices
        .iter()
        .filter(|device| device.rustdesk_id.as_deref() == Some("445566778"))
        .collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].rustdesk_version.as_deref(), Some("1.4.8"));
    assert!(matches[0].last_checkin_at.is_some());
}

#[tokio::test]
async fn enrollment_checkin_updates_existing_device_by_hostname() {
    let state = test_state().await;
    opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Existing host".to_string(),
            hostname: Some("ws-hostname-dup".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("seed device");
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "duplicate-hostname",
        None,
        None,
        None,
    )
    .await
    .expect("create token");
    let app = build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "112233445",
                        "hostname": "ws-hostname-dup",
                        "os_family": "linux",
                        "architecture": "aarch64"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let devices = opendesk::repository::devices::list_devices(&state.db)
        .await
        .expect("list devices");
    let hostname_matches: Vec<_> = devices
        .iter()
        .filter(|device| device.hostname.as_deref() == Some("ws-hostname-dup"))
        .collect();
    assert_eq!(hostname_matches.len(), 1);
    assert_eq!(
        hostname_matches[0].rustdesk_id.as_deref(),
        Some("112233445")
    );
}

#[tokio::test]
async fn devices_list_shows_last_checkin_column() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "list-checkin-token",
        None,
        None,
        None,
    )
    .await
    .expect("create token");
    let app = build_router(state);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "556677889",
                        "hostname": "list-checkin-host",
                        "os_family": "linux"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/devices")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("devices list");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Last check-in"));
    assert!(html.contains("556677889"));
    assert!(!html.contains(">-\n"));
}