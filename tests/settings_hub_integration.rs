mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, session_cookie_from_response, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::health::public_key_fingerprint;
use opendesk::domain::role::Role;
use opendesk::domain::server_config::ServerConfig;
use opendesk::repository::server_config::{load_server_config, save_server_config};
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

async fn get_html(app: &axum::Router, uri: &str, cookie: Option<&str>) -> (StatusCode, String) {
    let mut builder = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .expect("get");
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).expect("utf8"))
}

async fn post_form(
    app: &axum::Router,
    uri: &str,
    cookie: &str,
    body: &str,
) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(cookie, body)))
                .unwrap(),
        )
        .await
        .expect("post");
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).expect("utf8"))
}

fn fixture_config() -> ServerConfig {
    ServerConfig {
        id_server: "id.example.com".to_string(),
        relay_server: "relay.example.com".to_string(),
        api_server: "https://api.example.com".to_string(),
        public_key: "test-public-key".to_string(),
    }
}

#[tokio::test]
async fn settings_hub_requires_authentication() {
    let app = build_router(test_state().await);
    let (status, _) = get_html(&app, "/settings", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn settings_hub_renders_cards_for_admin() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = get_html(&app, "/settings", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("href=\"/settings/key\""));
    assert!(html.contains("href=\"/settings/relay\""));
    assert!(html.contains("href=\"/enrollment-tokens\""));
    assert!(html.contains("href=\"/deployment\""));
    assert!(html.contains("href=\"/settings/others\""));
    assert!(html.contains(">Key</h2>"));
    assert!(html.contains(">Relay</h2>"));
    assert!(html.contains(">Tokens</h2>"));
    assert!(html.contains(">Deployment</h2>"));
    assert!(html.contains(">Others</h2>"));
}

#[tokio::test]
async fn settings_pages_are_forbidden_for_read_only() {
    let state = test_state().await;
    create_user(&state.db, "viewer", "viewer-password", Role::READ_ONLY)
        .await
        .expect("create viewer");
    let app = build_router(state);
    let cookie = login_as(&app, "viewer", "viewer-password").await;
    for uri in [
        "/settings",
        "/settings/server-config",
        "/settings/key",
        "/settings/relay",
        "/settings/others",
    ] {
        let (status, _) = get_html(&app, uri, Some(&cookie)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
    }
}

#[tokio::test]
async fn server_config_page_keeps_combined_form() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = get_html(&app, "/settings/server-config", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Server Config</h1>"));
    assert!(html.contains("action=\"/settings/server-config\""));
    assert!(html.contains("name=\"id_server\""));
    assert!(html.contains("name=\"relay_server\""));
    assert!(html.contains("name=\"api_server\""));
    assert!(html.contains("name=\"public_key\""));
}

#[tokio::test]
async fn key_page_shows_fixture_fingerprint() {
    let state = test_state().await;
    save_server_config(&state.db, &fixture_config(), None)
        .await
        .expect("save config");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = get_html(&app, "/settings/key", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Key</h1>"));
    assert!(html.contains("action=\"/settings/key\""));
    assert!(html.contains("name=\"public_key\""));
    assert!(html.contains(&public_key_fingerprint("test-public-key")));
    assert!(!html.contains("name=\"id_server\""));
    assert!(!html.contains("name=\"relay_server\""));
}

#[tokio::test]
async fn relay_page_focuses_server_hosts() {
    let state = test_state().await;
    save_server_config(&state.db, &fixture_config(), None)
        .await
        .expect("save config");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = get_html(&app, "/settings/relay", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<h1>Relay</h1>"));
    assert!(html.contains("action=\"/settings/relay\""));
    assert!(html.contains("id.example.com"));
    assert!(html.contains("relay.example.com"));
    assert!(html.contains("https://api.example.com"));
    assert!(!html.contains("name=\"public_key\""));
}

#[tokio::test]
async fn others_page_states_non_enforcement() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = get_html(&app, "/settings/others", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("ALWAYS_USE_RELAY"));
    assert!(html.contains("session ACLs"));
    assert!(html.contains("public server fallback"));
    assert!(html.contains("href=\"/deployment\""));
    assert!(!html.contains("name=\"id_server\""));
}

#[tokio::test]
async fn key_post_updates_public_key_only() {
    let state = test_state().await;
    save_server_config(&state.db, &fixture_config(), None)
        .await
        .expect("save config");
    let app = build_router(state.clone());
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = post_form(
        &app,
        "/settings/key",
        &cookie,
        "public_key=rotated-test-key",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Server config saved"));
    assert!(html.contains(&public_key_fingerprint("rotated-test-key")));
    let saved = load_server_config(&state.db)
        .await
        .expect("load")
        .expect("config");
    assert_eq!(saved.public_key, "rotated-test-key");
    assert_eq!(saved.id_server, "id.example.com");
    assert_eq!(saved.relay_server, "relay.example.com");
    assert_eq!(saved.api_server, "https://api.example.com");
}

#[tokio::test]
async fn relay_post_updates_hosts_and_preserves_key() {
    let state = test_state().await;
    save_server_config(&state.db, &fixture_config(), None)
        .await
        .expect("save config");
    let app = build_router(state.clone());
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = post_form(
        &app,
        "/settings/relay",
        &cookie,
        "id_server=new-id.example.com&relay_server=new-relay.example.com&api_server=https://new-api.example.com",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Server config saved"));
    let saved = load_server_config(&state.db)
        .await
        .expect("load")
        .expect("config");
    assert_eq!(saved.id_server, "new-id.example.com");
    assert_eq!(saved.relay_server, "new-relay.example.com");
    assert_eq!(saved.api_server, "https://new-api.example.com");
    assert_eq!(saved.public_key, "test-public-key");
}

#[tokio::test]
async fn relay_post_redisplays_validation_error() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = post_form(
        &app,
        "/settings/relay",
        &cookie,
        "id_server=&relay_server=relay.example.com&api_server=",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("id_server must not be empty"));
    assert!(html.contains("class=\"error\""));
}

#[tokio::test]
async fn server_config_post_still_saves_combined_form() {
    let state = test_state().await;
    let app = build_router(state.clone());
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = post_form(
        &app,
        "/settings/server-config",
        &cookie,
        "id_server=combo-id.example.com&relay_server=combo-relay.example.com&api_server=https://combo-api.example.com&public_key=combo-test-key",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Server config saved"));
    assert!(html.contains("action=\"/settings/server-config\""));
    let saved = load_server_config(&state.db)
        .await
        .expect("load")
        .expect("config");
    assert_eq!(saved.id_server, "combo-id.example.com");
    assert_eq!(saved.relay_server, "combo-relay.example.com");
    assert_eq!(saved.api_server, "https://combo-api.example.com");
    assert_eq!(saved.public_key, "combo-test-key");
}
