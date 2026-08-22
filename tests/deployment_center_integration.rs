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

const OS_PAGES: [&str; 5] = [
    "/deployment/windows",
    "/deployment/linux",
    "/deployment/macos",
    "/deployment/android",
    "/deployment/ios",
];
const SCRIPT_EXPORTS: [&str; 3] = [
    "/deployment/windows.ps1",
    "/deployment/linux.sh",
    "/deployment/macos.sh",
];

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

async fn request_path(
    app: &axum::Router,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, String, Option<String>) {
    let mut builder = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .expect("request");
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        String::from_utf8(body.to_vec()).expect("utf8"),
        content_type,
    )
}

async fn deployment_html(app: &axum::Router, cookie: &str) -> (StatusCode, String) {
    let (status, body, _) = request_path(app, "/deployment", Some(cookie)).await;
    (status, body)
}

fn assert_no_secrets(body: &str) {
    assert!(!body.contains("client_access_token"));
    assert!(!body.contains("transport_introspection"));
    assert!(!body.contains("/deployment/windows.ps1?"));
    assert!(!body.contains("/deployment/linux.sh?"));
    assert!(!body.contains("/deployment/macos.sh?"));
}

async fn configured_app() -> (axum::Router, String) {
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
    (app, cookie)
}

#[tokio::test]
async fn deployment_center_requires_authentication() {
    let app = build_router(test_state().await);
    let (status, _, _) = request_path(&app, "/deployment", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn deployment_os_pages_and_scripts_require_authentication() {
    let app = build_router(test_state().await);
    for uri in OS_PAGES.iter().chain(SCRIPT_EXPORTS.iter()) {
        let (status, _, _) = request_path(&app, uri, None).await;
        assert_eq!(status, StatusCode::SEE_OTHER, "{uri}");
    }
}

#[tokio::test]
async fn deployment_center_shows_incomplete_state_and_os_pages() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html) = deployment_html(&app, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Setup incomplete"));
    for href in OS_PAGES {
        assert!(html.contains(&format!("href=\"{href}\"")), "{href}");
    }
    assert!(html.contains("iOS"));
    assert!(!html.contains("config="));
    assert!(!html.contains("rustdesk-host="));
    assert_no_secrets(&html);
}

#[tokio::test]
async fn configured_deployment_center_escapes_values_and_renders_qr() {
    let (app, cookie) = configured_app().await;
    let (status, html) = deployment_html(&app, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Official client configuration"));
    assert!(html.contains("config="));
    assert!(html.contains("aria-label=\"RustDesk server configuration QR code\""));
    assert!(html.contains("id.example.com&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("rustdesk-host="));
    assert_no_secrets(&html);
}

#[tokio::test]
async fn incomplete_os_pages_show_setup_notice_without_config() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    for uri in OS_PAGES {
        let (status, html, _) = request_path(&app, uri, Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(html.contains("Setup incomplete"), "{uri}");
        assert!(
            html.contains("https://github.com/rustdesk/rustdesk/releases"),
            "{uri}"
        );
        assert!(!html.contains("config="), "{uri}");
        assert!(!html.contains("PASTE_ENROLLMENT_TOKEN_VALUE"), "{uri}");
        assert!(!html.contains("rustdesk-host="), "{uri}");
        assert_no_secrets(&html);
    }
}

#[tokio::test]
async fn configured_windows_page_has_script_download_and_filename_fallback() {
    let (app, cookie) = configured_app().await;
    let (status, html, _) = request_path(&app, "/deployment/windows", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("View official download"));
    assert!(html.contains("config="));
    assert!(html.contains("href=\"/deployment/windows.ps1\""));
    assert!(!html.contains("/deployment/windows.ps1?"));
    assert!(html.contains("paste a newly created token"));
    assert!(html.contains("PASTE_ENROLLMENT_TOKEN_VALUE"));
    assert!(html.contains("Windows PowerShell script"));
    assert!(html.contains("rustdesk-host=id.example.com&lt;script&gt;,"));
    assert!(html.contains("id.example.com&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert_no_secrets(&html);
}

#[tokio::test]
async fn configured_linux_and_macos_pages_omit_filename_fallback() {
    let (app, cookie) = configured_app().await;
    for (uri, heading, href) in [
        (
            "/deployment/linux",
            "Linux shell script",
            "/deployment/linux.sh",
        ),
        (
            "/deployment/macos",
            "macOS shell script",
            "/deployment/macos.sh",
        ),
    ] {
        let (status, html, _) = request_path(&app, uri, Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        assert!(html.contains(heading), "{uri}");
        assert!(html.contains(&format!("href=\"{href}\"")), "{uri}");
        assert!(html.contains("paste a newly created token"), "{uri}");
        assert!(html.contains("config="), "{uri}");
        assert!(!html.contains("rustdesk-host="), "{uri}");
        assert!(
            !html.contains("Filename-based custom server fallback"),
            "{uri}"
        );
        assert_no_secrets(&html);
    }
}

#[tokio::test]
async fn configured_android_page_has_qr_and_manual_steps() {
    let (app, cookie) = configured_app().await;
    let (status, html, _) = request_path(&app, "/deployment/android", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Android operator setup"));
    assert!(html.contains("ID server"));
    assert!(html.contains("relay server"));
    assert!(html.contains("key"));
    assert!(html.contains("config="));
    assert!(html.contains("aria-label=\"RustDesk server configuration QR code\""));
    assert!(!html.contains("PASTE_ENROLLMENT_TOKEN_VALUE"));
    assert!(!html.contains("rustdesk-host="));
    assert_no_secrets(&html);
}

#[tokio::test]
async fn configured_ios_page_is_operator_only_and_uses_github_releases() {
    let (app, cookie) = configured_app().await;
    let (status, html, _) = request_path(&app, "/deployment/ios", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("iOS cannot be a controlled endpoint"));
    assert!(html.contains("https://github.com/rustdesk/rustdesk/releases"));
    assert!(html.contains("config="));
    assert!(html.contains("aria-label=\"RustDesk server configuration QR code\""));
    assert!(!html.contains("PASTE_ENROLLMENT_TOKEN_VALUE"));
    assert!(!html.contains("rustdesk-host="));
    assert!(!html.contains("/deployment/windows.ps1"));
    assert!(!html.contains("/deployment/linux.sh"));
    assert!(!html.contains("/deployment/macos.sh"));
    assert_no_secrets(&html);
}

#[tokio::test]
async fn script_exports_conflict_when_server_config_is_incomplete() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    for uri in SCRIPT_EXPORTS {
        let (status, _, _) = request_path(&app, uri, Some(&cookie)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}");
    }
}

#[tokio::test]
async fn windows_script_export_uses_plain_text_and_placeholder_token() {
    let (app, cookie) = configured_app().await;
    let (status, body, content_type) =
        request_path(&app, "/deployment/windows.ps1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/plain; charset=utf-8"));
    assert!(body.contains("$EnrollmentToken = 'PASTE_ENROLLMENT_TOKEN_VALUE'"));
    assert!(body.contains("$IdServer = 'id.example.com<script>'"));
    assert!(body.contains("/api/enrollments/check-in"));
    assert!(!body.contains("client_access_token"));
    assert!(!body.contains("transport_introspection"));
}

#[tokio::test]
async fn macos_script_export_uses_placeholder_token() {
    let (app, cookie) = configured_app().await;
    let (status, body, content_type) =
        request_path(&app, "/deployment/macos.sh", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/plain; charset=utf-8"));
    assert!(body.contains("ENROLLMENT_TOKEN='PASTE_ENROLLMENT_TOKEN_VALUE'"));
    assert!(body.contains("OS_FAMILY=\"macos\""));
    assert!(!body.contains("client_access_token"));
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
    let (status, _, _) = request_path(&app, "/deployment/ios", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = request_path(&app, "/deployment/windows.ps1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = request_path(&app, "/settings/server-config", Some(&cookie)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn signed_windows_installer_is_hidden_until_provisioned() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html, _) = request_path(&app, "/deployment/windows", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains("Download signed Windows installer"));
    let (status, _, _) = request_path(&app, "/deployment/windows/setup.exe", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn signed_windows_installer_is_served_when_provisioned() {
    let dir = std::env::temp_dir().join(format!("opendesk-signed-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("signed dir");
    std::fs::write(dir.join("windows-setup.exe"), b"signed-bytes").expect("exe");
    let mut state = test_state().await;
    state.signed_client_dir = Some(dir.clone());
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let (status, html, _) = request_path(&app, "/deployment/windows", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Download signed Windows installer"));
    assert!(html.contains("href=\"/deployment/windows/setup.exe\""));
    let (status, body, content_type) =
        request_path(&app, "/deployment/windows/setup.exe", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "signed-bytes");
    assert_eq!(
        content_type.as_deref(),
        Some("application/vnd.microsoft.portable-executable")
    );
    let (status, _, _) = request_path(&app, "/deployment/windows/setup.msi", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = request_path(&app, "/deployment/windows/setup.exe", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let _ = std::fs::remove_dir_all(&dir);
}
