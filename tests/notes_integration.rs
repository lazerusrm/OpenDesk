mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn device_list_shows_notes_and_search_matches_them() {
    let state = test_state().await;
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Notes Workstation".to_string(),
            notes: Some("Keep firmware updated weekly".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create device");

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/devices")
                .header("cookie", session_cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("devices list");
    assert_eq!(list_response.status(), StatusCode::OK);
    let body = list_response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Notes Workstation"));
    assert!(html.contains("Keep firmware updated weekly"));
    assert!(html.contains(r#"title="Keep firmware updated weekly""#));

    let search_response = app
        .oneshot(
            Request::builder()
                .uri("/devices?term=firmware")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("search devices");
    assert_eq!(search_response.status(), StatusCode::OK);
    let body = search_response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Notes Workstation"));
    let _ = device;
}