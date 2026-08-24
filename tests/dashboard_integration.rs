mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use tower::ServiceExt;

async fn body_text(response: axum::http::Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

#[tokio::test]
async fn unauthenticated_overview_redirects_to_login() {
    let app = build_router(test_state().await);
    for uri in ["/", "/dashboard"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .expect("overview");
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response
            .headers()
            .get("location")
            .expect("location")
            .to_str()
            .expect("location str");
        assert_eq!(location, "/login");
    }
}

#[tokio::test]
async fn admin_overview_shows_device_count() {
    let state = test_state().await;
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "Lab Workstation".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create device");
    let admin = opendesk::repository::users::find_user_by_username(&state.db, "admin")
        .await
        .expect("lookup admin")
        .expect("admin");
    opendesk::repository::device_visibility::replace_user_device_visibility_grants(
        &state.db,
        admin.user_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("grant visibility");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    for uri in ["/", "/dashboard"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("overview");
        assert_eq!(response.status(), StatusCode::OK);
        let html = body_text(response).await;
        assert!(html.contains("Overview"));
        assert!(html.contains(r#"data-count="devices"><strong>1</strong>"#));
        assert!(html.contains("Recently seen"));
        assert!(html.contains("Dashboard authorization is separate from RustDesk session access."));
        assert!(html.contains("Get started"));
        assert!(html.contains("/enrollment-tokens"));
        assert!(html.contains("/users"));
        assert!(!html.contains("online"));
        assert!(!html.contains("session connected"));
        assert!(!html.contains("rd.example.com"));
    }
}

#[tokio::test]
async fn empty_overview_shows_getting_started_links() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/dashboard")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("overview");
    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await;
    assert!(html.contains("Overview"));
    assert!(html.contains(r#"data-count="devices"><strong>0</strong>"#));
    assert!(html.contains("/devices/new"));
    assert!(html.contains("/users"));
    assert!(html.contains("/enrollment-tokens"));
    assert!(html.contains("/deployment"));
}
