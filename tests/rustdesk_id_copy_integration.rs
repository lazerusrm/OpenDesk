mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn device_list_renders_copy_button_for_rustdesk_id() {
    let state = test_state().await;
    opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Copy ID Workstation".to_string(),
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
    assert!(html.contains(r#"data-copy-text="123456789""#));
    assert!(html.contains("Copy ID Workstation"));
    assert!(html.contains(r#"<script src="/static/app.js" defer></script>"#));
}

#[tokio::test]
async fn device_list_omits_copy_button_without_rustdesk_id() {
    let state = test_state().await;
    opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "No ID Workstation".to_string(),
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
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("No ID Workstation"));
    assert!(!html.contains(r#"data-copy-text=""#));
    let copy_count = html.matches("class=\"copy-button\"").count();
    assert_eq!(copy_count, 0);
}

#[tokio::test]
async fn device_edit_renders_copy_button_for_rustdesk_id() {
    let state = test_state().await;
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Edit Copy Device".to_string(),
            rustdesk_id: Some("987654321".to_string()),
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
                .uri(format!("/devices/{}", device.device_uuid))
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("device edit");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains(r#"data-copy-input="rustdesk_id""#));
    assert!(html.contains(r#"value="987654321""#));
}