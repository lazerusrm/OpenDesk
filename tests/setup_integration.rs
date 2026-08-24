mod common;

use axum::body::Body;
use axum::http::{header::LOCATION, Request, StatusCode};
use common::{csrf_token_from_cookie, empty_test_state, session_cookie_from_response};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::config::{
    persist_public_base_url, resolve_client_token_hmac_key, resolve_public_base_url, HMAC_KEY_FILE,
    SQLITE_FILE,
};
use opendesk::repository::server_config::load_server_config;
use tower::ServiceExt;

async fn body_text(response: axum::response::Response) -> String {
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8")
}

fn setup_csrf_from_response(response: &axum::http::Response<Body>) -> String {
    let set_cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().expect("cookie header"))
        .find(|value| value.starts_with("opendesk_setup_csrf="))
        .expect("setup csrf cookie");
    set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

#[tokio::test]
async fn first_run_wizard_creates_admin_without_bootstrap_env() {
    let state = empty_test_state().await;
    let data_dir = state.data_dir.clone();
    let app = build_router(state.clone());
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("login redirect");
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        login
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/setup")
    );

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/setup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("setup page");
    assert_eq!(page.status(), StatusCode::OK);
    let csrf_cookie = setup_csrf_from_response(&page);
    let html = body_text(page).await;
    assert!(html.contains("admin"));
    assert!(html.contains("name=\"password\""));
    assert!(html.contains("name=\"public_base_url\""));
    assert!(html.contains("name=\"id_server\""));
    assert!(html.contains("name=\"relay_server\""));
    assert!(html.contains("name=\"api_server\""));
    assert!(html.contains("name=\"public_key\""));
    assert!(!html.contains("name=\"username\""));

    let csrf = csrf_cookie
        .strip_prefix("opendesk_setup_csrf=")
        .expect("csrf value");
    let submit = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/setup")
                .header("cookie", &csrf_cookie)
                .header("origin", "http://127.0.0.1:8080")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "password=wizard-pass&confirm_password=wizard-pass&public_base_url=http://127.0.0.1:8080&id_server=rd.example.com&relay_server=rd.example.com&api_server=https://rd.example.com&public_key=example-key&csrf_token={csrf}"
                )))
                .unwrap(),
        )
        .await
        .expect("setup submit");
    assert_eq!(submit.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        submit
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/login")
    );

    let stored = load_server_config(&state.db)
        .await
        .expect("load")
        .expect("config");
    assert_eq!(stored.id_server, "rd.example.com");
    assert_eq!(stored.relay_server, "rd.example.com");
    assert_eq!(stored.api_server, "https://rd.example.com");
    assert_eq!(stored.public_key, "example-key");
    assert_eq!(
        resolve_public_base_url(&data_dir, None),
        "http://127.0.0.1:8080"
    );

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("origin", "http://127.0.0.1:8080")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("username=admin&password=wizard-pass"))
                .unwrap(),
        )
        .await
        .expect("login");
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    let cookie = session_cookie_from_response(&login);
    assert!(cookie.contains("opendesk_session="));
    let _ = csrf_token_from_cookie(&cookie);
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn hmac_key_file_reuses_generated_bytes_and_fails_closed() {
    let dir = std::env::temp_dir().join(format!("opendesk-hmac-it-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    let first = resolve_client_token_hmac_key(&dir, None).expect("generate");
    assert!(first.len() >= 32);
    assert!(dir.join(HMAC_KEY_FILE).is_file());
    let second = resolve_client_token_hmac_key(&dir, None).expect("reuse");
    assert_eq!(first, second);

    let existing = std::env::temp_dir().join(format!("opendesk-hmac-db-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&existing).expect("dir");
    std::fs::write(existing.join(SQLITE_FILE), b"").expect("sqlite");
    assert!(resolve_client_token_hmac_key(&existing, None).is_err());
    persist_public_base_url(&dir, "https://rd.example.com").expect("url");
    assert_eq!(
        resolve_public_base_url(&dir, None),
        "https://rd.example.com"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&existing);
}

#[tokio::test]
async fn tree_does_not_vendor_rustdesk_source() {
    let output = std::process::Command::new("git")
        .args(["ls-files"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git ls-files");
    assert!(output.status.success());
    let listing = String::from_utf8_lossy(&output.stdout);
    assert!(!listing
        .lines()
        .any(|line| line.starts_with("upstream/rustdesk")));
    assert!(!listing
        .lines()
        .any(|line| line.starts_with("upstream/rustdesk-server")));
}
