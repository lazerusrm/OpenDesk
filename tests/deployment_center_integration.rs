mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, session_cookie_from_response, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::role::Role;
use opendesk::domain::server_config::ServerConfig;
use opendesk::repository::server_config::save_server_config;
use opendesk::repository::users::create_user;
use tower::ServiceExt;

async fn login_as(app: &axum::Router, username: &str, password: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("origin", "http://127.0.0.1:8080")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "username={username}&password={password}"
                )))
                .unwrap(),
        )
        .await
        .expect("login");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    session_cookie_from_response(&response)
}

async fn deployment_html(app: &axum::Router, cookie: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/deployment")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("deployment");
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).expect("utf8"))
}

#[tokio::test]
async fn deployment_center_requires_authentication() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/deployment")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("deployment");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn deployment_center_shows_incomplete_state_and_generic_downloads() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = deployment_html(&app, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Setup incomplete"));
    assert!(html.contains("https://github.com/rustdesk/rustdesk/releases"));
    assert!(!html.contains("config="));
    assert!(!html.contains("client_access_token"));
    assert!(!html.contains("transport_introspection"));
}

#[tokio::test]
async fn configured_deployment_center_escapes_values_and_renders_qr() {
    let state = test_state().await;
    save_server_config(
        &state.db,
        &ServerConfig {
            id_server: "id.example.com<script>".to_string(),
            relay_server: "relay.example.com".to_string(),
            api_server: "https://console.example.com".to_string(),
            public_key: "public-key-\"safe\"".to_string(),
        },
        None,
    )
    .await
    .expect("save config");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = deployment_html(&app, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Official client configuration"));
    assert!(html.contains("config="));
    assert!(html.contains("aria-label=\"RustDesk server configuration QR code\""));
    assert!(html.contains("id.example.com&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("client_access_token"));
}

#[tokio::test]
async fn read_only_role_can_view_but_cannot_configure_server() {
    let state = test_state().await;
    create_user(&state.db, "viewer", "viewer-password", Role::READ_ONLY)
        .await
        .expect("create viewer");
    let app = build_router(state);
    let cookie = login_as(&app, "viewer", "viewer-password").await;
    let (status, _) = deployment_html(&app, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/settings/server-config")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("server config");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
