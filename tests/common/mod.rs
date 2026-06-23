use axum::body::Body;
use axum::http::{Request, StatusCode};
use opendesk::{build_router, AppState};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;

pub async fn test_state() -> AppState {
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

pub fn session_cookie_from_response(response: &axum::http::Response<Body>) -> String {
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

pub async fn login_and_get_session_cookie(app: &axum::Router) -> String {
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