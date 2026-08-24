mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, session_cookie_from_response, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::repository::audit_events::list_audit_events;
use tower::ServiceExt;

async fn body_text(response: axum::http::Response<Body>) -> String {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).expect("utf8")
}

async fn login(app: &axum::Router, username: &str, password: &str) -> axum::http::Response<Body> {
    app.clone()
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
        .expect("login")
}

#[tokio::test]
async fn account_page_requires_auth() {
    let app = build_router(test_state().await);
    let unauthenticated = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/account")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("unauthenticated account");
    assert_eq!(unauthenticated.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        unauthenticated
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/login")
    );

    let cookie = login_and_get_session_cookie(&app).await;
    let authenticated = app
        .oneshot(
            Request::builder()
                .uri("/account")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("authenticated account");
    assert_eq!(authenticated.status(), StatusCode::OK);
    let html = body_text(authenticated).await;
    assert!(html.contains("admin"));
    assert!(html.contains("href=\"/account\""));
    assert!(html.contains("name=\"current_password\""));
    assert!(html.contains("name=\"new_password\""));
    assert!(html.contains("name=\"confirm_password\""));
}

#[tokio::test]
async fn account_password_change_rejects_wrong_current_password() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/account/password")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "current_password=wrong-password&new_password=new-password&confirm_password=new-password",
                )))
                .unwrap(),
        )
        .await
        .expect("wrong current password");
    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response).await;
    assert!(html.contains("current password is incorrect"));

    let still_valid = login(&app, "admin", "test-password").await;
    assert_eq!(still_valid.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn account_password_change_updates_password_and_requires_login() {
    let state = test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/account/password")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "current_password=test-password&new_password=new-password&confirm_password=new-password",
                )))
                .unwrap(),
        )
        .await
        .expect("change password");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("redirect");
    assert_eq!(location, "/login?login=password-updated");

    let notice = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(location)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("login notice");
    assert_eq!(notice.status(), StatusCode::OK);
    let notice_html = body_text(notice).await;
    assert!(notice_html.contains("class=\"muted\""));
    assert!(notice_html.contains("Password updated"));

    let old_password = login(&app, "admin", "test-password").await;
    assert_eq!(old_password.status(), StatusCode::OK);
    let old_html = body_text(old_password).await;
    assert!(old_html.contains("Invalid username or password"));

    let new_password = login(&app, "admin", "new-password").await;
    assert_eq!(new_password.status(), StatusCode::SEE_OTHER);
    assert!(!session_cookie_from_response(&new_password).is_empty());

    let stale_session = app
        .oneshot(
            Request::builder()
                .uri("/account")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("stale session");
    assert_eq!(stale_session.status(), StatusCode::SEE_OTHER);

    let events = list_audit_events(&db, 100).await.expect("audit");
    assert!(events.iter().any(|event| {
        event.action == "account_password_change"
            && event.object_type == "user"
            && event.outcome == "success"
            && event.detail_json.is_none()
    }));
}

#[tokio::test]
async fn account_password_change_requires_csrf() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;

    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/account/password")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "current_password=test-password&new_password=new-password&confirm_password=new-password",
                ))
                .unwrap(),
        )
        .await
        .expect("missing csrf");
    assert_eq!(missing.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let invalid = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/account/password")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "csrf_token=invalid&current_password=test-password&new_password=new-password&confirm_password=new-password",
                ))
                .unwrap(),
        )
        .await
        .expect("invalid csrf");
    assert_eq!(invalid.status(), StatusCode::FORBIDDEN);
}
