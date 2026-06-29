mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::health::public_key_fingerprint;
use opendesk::domain::server_config::default_server_config;
use opendesk::repository::server_config::save_server_config;
use tower::ServiceExt;

async fn seed_server_config(state: &opendesk::AppState) {
    let mut config = default_server_config();
    config.public_key = "test-public-key".to_string();
    save_server_config(&state.db, &config, None)
        .await
        .expect("save server config");
}

#[tokio::test]
async fn device_list_renders_connection_helper_copy_buttons() {
    let state = test_state().await;
    seed_server_config(&state).await;
    opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Helper Workstation".to_string(),
            rustdesk_id: Some("123456789".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create device");

    let app = build_router(state);
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
    assert!(html.contains("Copy default"));
    assert!(html.contains("Copy explicit"));
    assert!(html.contains(r#"data-copy-text="123456789""#));
    assert!(html.contains(
        r#"data-copy-text="123456789@rd.example.com:21117?key=test-public-key""#
    ));
}

#[tokio::test]
async fn deployment_page_renders_macos_script_and_filename_fallback() {
    let state = test_state().await;
    seed_server_config(&state).await;

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/deployment")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("deployment page");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("macOS shell script"));
    assert!(html.contains("OS_FAMILY=") && html.contains("macos"));
    assert!(html.contains("rustdesk-host=rd.example.com,"));
    assert!(html.contains("Official RustDesk clients"));
    assert!(html.contains("https://github.com/rustdesk/rustdesk/releases"));
}

#[tokio::test]
async fn status_dashboard_renders_health_targets_and_fingerprint() {
    let state = test_state().await;
    seed_server_config(&state).await;

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/status")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("status page");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Public key fingerprint"));
    assert!(html.contains(&public_key_fingerprint("test-public-key")));
    assert!(html.contains("tcp:rd.example.com:21116"));
    assert!(html.contains("tcp:rd.example.com:21117"));
    assert!(html.contains("dns:rd.example.com"));
}

#[tokio::test]
async fn status_dashboard_requires_auth() {
    let state = test_state().await;
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("status page");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}