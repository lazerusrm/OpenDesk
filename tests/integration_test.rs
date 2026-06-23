use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use opendesk::{build_router, AppState};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

async fn test_state() -> AppState {
    let db = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations").run(&db).await.expect("migrate");
    opendesk::repository::users::create_user(&db, "admin", "test-password", "admin")
        .await
        .expect("bootstrap user");
    AppState {
        db,
        cookie_secure: false,
        public_base_url: "http://127.0.0.1:8080".to_string(),
    }
}

fn session_cookie_from_response(response: &axum::http::Response<Body>) -> String {
    let set_cookie = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().expect("cookie header"))
        .find(|value| value.starts_with("opendesk_session="))
        .expect("session cookie");
    set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn login_and_get_session_cookie(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("username=admin&password=test-password"))
                .unwrap(),
        )
        .await
        .expect("login response");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    session_cookie_from_response(&response)
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn login_page_renders_opendesk_form() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(Request::builder().uri("/login").body(Body::empty()).unwrap())
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("OpenDesk"));
    assert!(html.contains("Admin Login"));
}

#[tokio::test]
async fn device_update_via_handler_preserves_enrollment_metadata() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "metadata-token",
        None,
        None,
        None,
    )
    .await
    .expect("create token");

    let app = build_router(state.clone());
    let checkin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "554433221",
                        "hostname": "dev-host",
                        "os_family": "linux",
                        "os_version": "test-os",
                        "architecture": "x86_64",
                        "rustdesk_version": "1.4.8"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    assert_eq!(checkin.status(), StatusCode::NO_CONTENT);

    let device = opendesk::repository::devices::find_device_by_rustdesk_id(
        &state.db,
        "554433221",
    )
    .await
    .expect("lookup")
    .expect("device");

    let session_cookie = login_and_get_session_cookie(&app).await;
    let update = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/devices/{}", device.device_uuid))
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie)
                .body(Body::from(
                    "alias=Renamed+device&notes=operator+note&rustdesk_id=&hostname=&owner=",
                ))
                .unwrap(),
        )
        .await
        .expect("device update");
    assert_eq!(update.status(), StatusCode::SEE_OTHER);

    let updated = opendesk::repository::devices::find_device_by_uuid(
        &state.db,
        device.device_uuid,
    )
    .await
    .expect("reload")
    .expect("updated device");

    assert_eq!(updated.alias, "Renamed device");
    assert_eq!(updated.os_family.as_deref(), Some("linux"));
    assert_eq!(updated.architecture.as_deref(), Some("x86_64"));
    assert_eq!(updated.rustdesk_version.as_deref(), Some("1.4.8"));
    assert_eq!(updated.notes.as_deref(), Some("operator note"));
}

#[tokio::test]
async fn enrollment_checkin_creates_device_with_valid_token() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "integration-token",
        None,
        None,
        None,
    )
    .await
    .expect("create token");

    let db = state.db.clone();
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "998877665",
                        "hostname": "dev-host",
                        "os_family": "linux",
                        "architecture": "x86_64"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let device = opendesk::repository::devices::find_device_by_rustdesk_id(&db, "998877665")
        .await
        .expect("lookup");
    assert!(device.is_some());
}