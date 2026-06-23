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
    }
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