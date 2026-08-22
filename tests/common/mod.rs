use axum::body::Body;
use axum::http::{Request, StatusCode};
use opendesk::AppState;
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

pub async fn test_state() -> AppState {
    let db = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("migrate");
    opendesk::repository::users::create_user(&db, "admin", "test-password", "admin")
        .await
        .expect("bootstrap user");
    AppState {
        db,
        cookie_secure: false,
        public_base_url: "http://127.0.0.1:8080".to_string(),
        backup_schedule: None,
        backup_destination_configured: false,
        client_token_hmac_key: vec![7; 32],
        transport_introspection_key: Some(vec![9; 32]),
        rustdesk_download_windows_url: None,
        rustdesk_download_macos_url: None,
        rustdesk_download_linux_url: None,
        rustdesk_download_android_url: None,
        signed_client_dir: None,
        login_throttle: std::sync::Arc::new(std::sync::Mutex::new(
            opendesk::login_throttle::LoginThrottle::default(),
        )),
        onboard_guard: std::sync::Arc::new(std::sync::Mutex::new(
            opendesk::onboard_guard::OnboardGuard::default(),
        )),
        pending_onboard_totp: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
    }
}

pub fn session_cookie_from_response(response: &axum::http::Response<Body>) -> String {
    let mut cookies = Vec::new();
    for name in ["opendesk_session", "opendesk_csrf"] {
        let set_cookie = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().expect("cookie header"))
            .find(|value| value.starts_with(&format!("{name}=")))
            .expect("cookie");
        cookies.push(
            set_cookie
                .split(';')
                .next()
                .expect("cookie pair")
                .to_string(),
        );
    }
    cookies.join("; ")
}

#[allow(dead_code)]
pub fn csrf_token_from_cookie(cookie: &str) -> &str {
    cookie
        .split(';')
        .find_map(|part| part.trim().strip_prefix("opendesk_csrf="))
        .expect("csrf cookie")
}

#[allow(dead_code)]
pub fn form_with_csrf(cookie: &str, body: &str) -> String {
    format!("{body}&csrf_token={}", csrf_token_from_cookie(cookie))
}

pub async fn login_and_get_session_cookie(app: &axum::Router) -> String {
    login_and_get_session_cookie_with_origin(app, "http://127.0.0.1:8080").await
}

#[allow(dead_code)]
pub async fn login_and_get_session_cookie_as(
    app: &axum::Router,
    username: &str,
    password: &str,
) -> String {
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
        .expect("login response");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    session_cookie_from_response(&response)
}

pub async fn login_and_get_session_cookie_with_origin(app: &axum::Router, origin: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("origin", origin)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("username=admin&password=test-password"))
                .unwrap(),
        )
        .await
        .expect("login response");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    session_cookie_from_response(&response)
}
