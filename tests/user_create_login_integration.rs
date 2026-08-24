mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, login_and_get_session_cookie_as, test_state,
};
use opendesk::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn created_user_can_login_with_exact_untrimmed_password() {
    let state = test_state().await;
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "username=newops&password=securepass1&role=operator",
                )))
                .unwrap(),
        )
        .await
        .expect("create user");
    assert_eq!(create.status(), StatusCode::SEE_OTHER);

    let session = login_and_get_session_cookie_as(&app, "newops", "securepass1").await;
    assert!(session.contains("opendesk_session="));
}

#[tokio::test]
async fn created_user_password_whitespace_matches_login() {
    let state = test_state().await;
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "username=spaceops&password=%20securepass1%20&role=operator",
                )))
                .unwrap(),
        )
        .await
        .expect("create user");
    assert_eq!(create.status(), StatusCode::SEE_OTHER);

    let session = login_and_get_session_cookie_as(&app, "spaceops", "%20securepass1%20").await;
    assert!(session.contains("opendesk_session="));

    let trimmed_login = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("origin", "http://127.0.0.1:8080")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("username=spaceops&password=securepass1"))
                .unwrap(),
        )
        .await
        .expect("trimmed login");
    assert_eq!(trimmed_login.status(), StatusCode::OK);
}
