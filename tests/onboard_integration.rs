mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, login_and_get_session_cookie_as, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::domain::role::Role;
use opendesk::domain::server_config::ServerConfig;
use opendesk::repository::device_visibility::is_device_visible_to_user;
use opendesk::repository::devices::find_device_by_rustdesk_id;
use opendesk::repository::enrollment_tokens::create_enrollment_token;
use opendesk::repository::server_config::save_server_config;
use opendesk::repository::users::{count_users, create_user, find_user_by_username};
use serde_json::json;
use tower::ServiceExt;

async fn body_text(
    response: axum::http::Response<Body>,
) -> (StatusCode, String, axum::http::HeaderMap) {
    let status = response.status();
    let headers = response.headers().clone();
    let body = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    (status, body, headers)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String, axum::http::HeaderMap) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .expect("get");
    body_text(response).await
}

async fn configure(state: &opendesk::AppState) {
    save_server_config(
        &state.db,
        &ServerConfig {
            id_server: "id.example.com".to_string(),
            relay_server: "relay.example.com".to_string(),
            api_server: "https://console.example.com".to_string(),
            public_key: "public-key".to_string(),
        },
        None,
    )
    .await
    .expect("save config");
}

fn extract_onboard_token(html: &str) -> String {
    html.split("/onboard/")
        .nth(1)
        .expect("onboard path")
        .chars()
        .take_while(|ch| ch.is_ascii_hexdigit())
        .collect()
}

#[tokio::test]
async fn onboard_index_does_not_enroll_or_download() {
    let app = build_router(test_state().await);
    let (status, html, headers) = get(&app, "/onboard").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Enter the technician code"));
    assert!(html.contains("name=\"code\""));
    assert!(!html.contains("Technician (optional)"));
    assert!(!html.contains("does not create an account"));
    assert!(!html.contains("Download Windows installer"));
    assert!(!html.contains("irm "));
    assert!(!html.contains("sudo bash"));
    assert!(!html.contains("name=\"password\""));
    assert_eq!(
        headers
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    assert_eq!(
        headers
            .get("referrer-policy")
            .and_then(|value| value.to_str().ok()),
        Some("same-origin")
    );
}

#[tokio::test]
async fn invalid_onboard_token_is_generic_not_found() {
    let app = build_router(test_state().await);
    for uri in [
        "/onboard/not-a-token",
        "/onboard/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        let (status, html, _) = get(&app, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert!(html.contains("invalid or has expired"), "{uri}");
        assert!(!html.contains("Download Windows installer"), "{uri}");
        let (script_status, _, _) = get(&app, &format!("{uri}/linux.sh")).await;
        assert_eq!(script_status, StatusCode::NOT_FOUND, "{uri} script");
    }
}

#[tokio::test]
async fn valid_onboard_link_is_public_and_grants_issuer_only() {
    let state = test_state().await;
    configure(&state).await;
    opendesk::repository::devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "SECRETDEVICE".to_string(),
            rustdesk_id: Some("111000111".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("seed device");
    let issuer = create_user(&state.db, "operator", "test-password", Role::OPERATOR)
        .await
        .expect("operator");
    let other = create_user(&state.db, "other-op", "test-password", Role::OPERATOR)
        .await
        .expect("other");
    let users_before = count_users(&state.db).await.expect("count");
    let created =
        create_enrollment_token(&state.db, "field-visit", None, None, Some(issuer.user_uuid))
            .await
            .expect("token");
    let dir = std::env::temp_dir().join(format!("opendesk-onboard-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("windows-setup.exe"), b"MZ-signed").expect("exe");
    let mut state = state;
    state.signed_client_dir = Some(dir.clone());
    let db = state.db.clone();
    let app = build_router(state);
    let token = &created.token_value;
    let page_uri = format!("/onboard/{token}");
    let (status, html, headers) = get(&app, &page_uri).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("does not give you access to other computers"));
    assert!(html.contains("Download Windows installer"));
    assert!(html.contains(&format!("/onboard/{token}/windows/setup.exe")));
    assert!(html.contains(&format!("/onboard/{token}/windows.ps1")));
    assert!(html.contains(&format!("/onboard/{token}/linux.sh")));
    assert!(html.contains("sudo bash"));
    assert!(html.contains("irm "));
    assert!(!html.contains("SECRETDEVICE"));
    assert!(!html.contains("Sign in"));
    assert!(!headers.contains_key("set-cookie"));
    assert_eq!(
        headers
            .get("x-robots-tag")
            .and_then(|value| value.to_str().ok()),
        Some("noindex")
    );

    let (status, script, _) = get(&app, &format!("/onboard/{token}/linux.sh")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(script.contains(&format!("ENROLLMENT_TOKEN='{token}'")));
    assert!(script.contains("/api/enrollments/check-in"));
    assert!(script.contains("id.example.com"));

    let (status, setup, headers) = get(&app, &format!("/onboard/{token}/windows/setup.exe")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(setup.starts_with("MZ-signed"));
    assert!(setup.contains("OPENDESK_ONBOARD_V1"));
    assert!(setup.contains(token));
    assert_eq!(
        headers
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/vnd.microsoft.portable-executable")
    );

    let checkin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": token,
                        "rustdesk_id": "998877001",
                        "hostname": "field-pc",
                        "os_family": "windows"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    assert_eq!(checkin.status(), StatusCode::NO_CONTENT);
    let device = find_device_by_rustdesk_id(&db, "998877001")
        .await
        .expect("lookup")
        .expect("device");
    assert!(
        is_device_visible_to_user(&db, issuer.user_uuid, device.device_uuid)
            .await
            .expect("issuer vis")
    );
    assert!(
        !is_device_visible_to_user(&db, other.user_uuid, device.device_uuid)
            .await
            .expect("other vis")
    );
    let admin = find_user_by_username(&db, "admin")
        .await
        .expect("admin lookup")
        .expect("admin");
    assert!(
        is_device_visible_to_user(&db, admin.user_uuid, device.device_uuid)
            .await
            .expect("admin vis")
    );
    assert_eq!(count_users(&db).await.expect("count after"), users_before);
    let devices = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("devices");
    assert_eq!(devices.status(), StatusCode::SEE_OTHER);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn enrollment_create_shows_onboard_url_once() {
    let state = test_state().await;
    configure(&state).await;
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/enrollment-tokens")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "label=sendable-link&expires_in_days=7",
                )))
                .unwrap(),
        )
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::OK);
    let html = String::from_utf8(
        created
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    assert!(html.contains("id=\"created-onboard-url\""));
    let token = extract_onboard_token(&html);
    assert_eq!(token.len(), 64);
    assert!(html.contains(&format!("http://127.0.0.1:8080/onboard/{token}")));
    let (status, page, _) = get(&app, &format!("/onboard/{token}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("Install remote support"));
}

#[tokio::test]
async fn revoked_onboard_link_stops_working() {
    let state = test_state().await;
    configure(&state).await;
    let admin = find_user_by_username(&state.db, "admin")
        .await
        .expect("admin")
        .expect("admin");
    let created =
        create_enrollment_token(&state.db, "revoke-me", None, None, Some(admin.user_uuid))
            .await
            .expect("token");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let token = created.token_value.clone();
    let (status, _, _) = get(&app, &format!("/onboard/{token}")).await;
    assert_eq!(status, StatusCode::OK);
    let revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/enrollment-tokens/{}/revoke",
                    created.record.enrollment_token_uuid
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "")))
                .unwrap(),
        )
        .await
        .expect("revoke");
    assert_eq!(revoke.status(), StatusCode::OK);
    let (status, html, _) = get(&app, &format!("/onboard/{token}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(html.contains("invalid or has expired"));
}

#[tokio::test]
async fn signed_installer_without_onboard_token_still_requires_login() {
    let dir = std::env::temp_dir().join(format!("opendesk-deploy-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("windows-setup.exe"), b"MZ-signed").expect("exe");
    let mut state = test_state().await;
    state.signed_client_dir = Some(dir.clone());
    let app = build_router(state);
    let (status, _, _) = get(&app, "/deployment/windows/setup.exe").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let cookie = login_and_get_session_cookie_as(&app, "admin", "test-password").await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/deployment/windows/setup.exe")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("auth download");
    assert_eq!(response.status(), StatusCode::OK);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn token_without_issuer_does_not_grant_operator_visibility() {
    let state = test_state().await;
    let operator = create_user(&state.db, "operator", "test-password", Role::OPERATOR)
        .await
        .expect("operator");
    let created = create_enrollment_token(&state.db, "legacy", None, None, None)
        .await
        .expect("token");
    let app = build_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "110011001",
                        "hostname": "legacy-pc"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let device = find_device_by_rustdesk_id(&state.db, "110011001")
        .await
        .expect("lookup")
        .expect("device");
    assert!(
        !is_device_visible_to_user(&state.db, operator.user_uuid, device.device_uuid)
            .await
            .expect("vis")
    );
}
