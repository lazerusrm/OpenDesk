mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn default_device_list_hides_archived_devices() {
    let state = test_state().await;
    let active = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Active List Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create active device");
    let archived = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Archived List Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create archived device");
    opendesk::repository::devices::set_device_archived(&state.db, archived.device_uuid, true)
        .await
        .expect("archive device");

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
    assert!(html.contains("Active List Device"));
    assert!(!html.contains("Archived List Device"));
    let _ = active;
}