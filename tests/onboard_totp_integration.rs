mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{form_with_csrf, login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::onboard_totp::{decode_base32, totp_code, verify_totp};
use opendesk::domain::role::Role;
use opendesk::domain::server_config::ServerConfig;
use opendesk::repository::device_visibility::is_device_visible_to_user;
use opendesk::repository::devices::find_device_by_rustdesk_id;
use opendesk::repository::onboard_totp::save_user_onboard_totp;
use opendesk::repository::server_config::save_server_config;
use opendesk::repository::users::create_user;
use serde_json::json;
use time::OffsetDateTime;
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

fn set_cookie(headers: &axum::http::HeaderMap, name: &str) -> String {
    headers
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().expect("cookie"))
        .find(|value| value.starts_with(&format!("{name}=")))
        .unwrap_or_else(|| panic!("{name} cookie"))
        .split(';')
        .next()
        .expect("pair")
        .to_string()
}

fn onboard_cookie(headers: &axum::http::HeaderMap) -> String {
    set_cookie(headers, "opendesk_onboard")
}

fn csrf_from_page(html: &str) -> String {
    html.split("name=\"csrf_token\"")
        .nth(1)
        .and_then(|chunk| chunk.split("value=\"").nth(1))
        .and_then(|chunk| chunk.split('"').next())
        .expect("csrf")
        .to_string()
}

#[tokio::test]
async fn totp_code_unlocks_onboard_for_issuer_only() {
    let state = test_state().await;
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
    .expect("config");
    let operator = create_user(&state.db, "operator", "test-password", Role::OPERATOR)
        .await
        .expect("operator");
    let other = create_user(&state.db, "other-op", "test-password", Role::OPERATOR)
        .await
        .expect("other");
    let secret = b"12345678901234567890";
    save_user_onboard_totp(&state.db, operator.user_uuid, secret)
        .await
        .expect("enroll");
    let db = state.db.clone();
    let app = build_router(state);
    let now = OffsetDateTime::now_utc();
    let code = totp_code(secret, now.unix_timestamp());
    assert!(verify_totp(secret, &code, now).is_some());

    let form_page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/onboard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("form");
    assert_eq!(form_page.status(), StatusCode::OK);
    let csrf_cookie = set_cookie(form_page.headers(), "opendesk_onboard_csrf");
    let form_html = String::from_utf8(
        form_page
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    let csrf = csrf_from_page(&form_html);

    let missing_csrf = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/onboard")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("code={code}")))
                .unwrap(),
        )
        .await
        .expect("csrf");
    assert_eq!(missing_csrf.status(), StatusCode::OK);

    let unlocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/onboard")
                .header("cookie", &csrf_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("code={code}&csrf_token={csrf}")))
                .unwrap(),
        )
        .await
        .expect("unlock");
    assert_eq!(unlocked.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        unlocked
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/onboard")
    );
    let cookie = onboard_cookie(unlocked.headers());
    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/onboard")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("page");
    let (status, html, _) = body_text(page).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("does not give you access to other computers"));
    assert!(html.contains("/onboard/") && html.contains("/linux.sh"));

    let replay = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/onboard")
                .header("cookie", &csrf_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("code={code}&csrf_token={csrf}")))
                .unwrap(),
        )
        .await
        .expect("replay");
    let (status, html, _) = body_text(replay).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("already used") || html.contains("not valid"));

    let token = html_token_from_cookie(&cookie);
    let checkin = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": token,
                        "rustdesk_id": "556677001",
                        "hostname": "field-laptop"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    assert_eq!(checkin.status(), StatusCode::NO_CONTENT);
    let device = find_device_by_rustdesk_id(&db, "556677001")
        .await
        .expect("lookup")
        .expect("device");
    assert!(
        is_device_visible_to_user(&db, operator.user_uuid, device.device_uuid)
            .await
            .expect("issuer")
    );
    assert!(
        !is_device_visible_to_user(&db, other.user_uuid, device.device_uuid)
            .await
            .expect("other")
    );
}

fn html_token_from_cookie(cookie: &str) -> String {
    cookie
        .strip_prefix("opendesk_onboard=")
        .expect("prefix")
        .to_string()
}

#[tokio::test]
async fn account_enrolls_onboard_authenticator() {
    let state = test_state().await;
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/onboard-setup/start")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "")))
                .unwrap(),
        )
        .await
        .expect("start");
    assert_eq!(start.status(), StatusCode::OK);
    let html = String::from_utf8(
        start
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    assert!(html.contains("id=\"onboard-totp-secret\""));
    let secret_b32 = html
        .split("id=\"onboard-totp-secret\"")
        .nth(1)
        .and_then(|chunk| chunk.split("value=\"").nth(1))
        .and_then(|chunk| chunk.split('"').next())
        .expect("secret");
    let secret = decode_base32(secret_b32).expect("base32");
    let code = totp_code(&secret, OffsetDateTime::now_utc().unix_timestamp());
    let confirm = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/onboard-setup/confirm")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, &format!("code={code}"))))
                .unwrap(),
        )
        .await
        .expect("confirm");
    assert_eq!(confirm.status(), StatusCode::OK);
    let confirmed = String::from_utf8(
        confirm
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    assert!(confirmed.contains("authenticator enrolled"));
    assert!(confirmed.contains("Revoke authenticator"));
}

#[tokio::test]
async fn onboard_setup_page_is_in_console_nav() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let page = app
        .oneshot(
            Request::builder()
                .uri("/onboard-setup")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("page");
    assert_eq!(page.status(), StatusCode::OK);
    let html = String::from_utf8(
        page.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    assert!(html.contains("href=\"/onboard-setup\""));
    assert!(html.contains("Set up authenticator"));
    assert!(html.contains("Onboard codes"));
}
