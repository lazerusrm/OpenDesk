mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    csrf_token_from_cookie, form_with_csrf, login_and_get_session_cookie,
    login_and_get_session_cookie_as, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::role::Role;
use opendesk::repository::audit_events::list_audit_events;
use opendesk::repository::users::{create_user, find_user_by_username};
use tower::ServiceExt;

async fn html(app: &axum::Router, cookie: &str, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("page");
    let status = response.status();
    let body = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    (status, body)
}

#[tokio::test]
async fn failed_web_login_is_audited_and_lockout_trips() {
    let state = test_state().await;
    let app = build_router(state.clone());
    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header("origin", "http://127.0.0.1:8080")
                    .header("x-forwarded-for", "203.0.113.10")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("username=admin&password=wrong-password"))
                    .unwrap(),
            )
            .await
            .expect("failed login");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let locked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("origin", "http://127.0.0.1:8080")
                .header("x-forwarded-for", "203.0.113.10")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("username=admin&password=test-password"))
                .unwrap(),
        )
        .await
        .expect("lockout");
    assert_eq!(locked.status(), StatusCode::OK);
    let body = String::from_utf8(
        locked
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("Too many sign-in attempts"));
    let events = list_audit_events(&state.db, 20).await.expect("audit");
    assert!(events.iter().any(|event| event.action == "login"
        && event.outcome == "failure"
        && event.source == "web"));
}

#[tokio::test]
async fn disable_user_revokes_session_and_blocks_dashboard() {
    let state = test_state().await;
    let operator = create_user(
        &state.db,
        "shop-operator",
        "operator-password",
        Role::OPERATOR,
    )
    .await
    .expect("operator");
    let app = build_router(state.clone());
    let operator_cookie =
        login_and_get_session_cookie_as(&app, "shop-operator", "operator-password").await;
    let (status, _) = html(&app, &operator_cookie, "/devices").await;
    assert_eq!(status, StatusCode::OK);

    let admin_cookie = login_and_get_session_cookie(&app).await;
    let (status, users_html) = html(&app, &admin_cookie, "/users").await;
    assert_eq!(status, StatusCode::OK);
    assert!(users_html.contains("Disable user"));
    let disabled = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users/disable")
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &admin_cookie,
                    &format!("user_uuid={}", operator.user_uuid),
                )))
                .unwrap(),
        )
        .await
        .expect("disable");
    assert_eq!(disabled.status(), StatusCode::SEE_OTHER);
    let (status, _) = html(&app, &operator_cookie, "/devices").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let user = find_user_by_username(&state.db, "shop-operator")
        .await
        .expect("lookup")
        .expect("operator");
    assert_eq!(user.activation_state, "disabled");
}

#[tokio::test]
async fn operator_cannot_disable_users_or_delete_address_book_entries() {
    let state = test_state().await;
    create_user(
        &state.db,
        "shop-operator",
        "operator-password",
        Role::OPERATOR,
    )
    .await
    .expect("operator");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie_as(&app, "shop-operator", "operator-password").await;
    let disable = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/users/disable")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "user_uuid=00000000-0000-0000-0000-000000000000",
                )))
                .unwrap(),
        )
        .await
        .expect("disable");
    assert_eq!(disable.status(), StatusCode::FORBIDDEN);
    let (status, books) = html(&app, &cookie, "/address-books").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!books.contains("Delete entry"));
}

fn has_clickjacking_and_nosniff(headers: &axum::http::HeaderMap) -> bool {
    let nosniff = headers
        .get("x-content-type-options")
        .and_then(|value| value.to_str().ok())
        == Some("nosniff");
    let frame = headers
        .get("x-frame-options")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.eq_ignore_ascii_case("DENY") || value.eq_ignore_ascii_case("SAMEORIGIN")
        });
    let csp = headers
        .get("content-security-policy")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.contains("frame-ancestors 'none'") || value.contains("frame-ancestors 'self'")
        });
    nosniff && (frame || csp)
}

#[tokio::test]
async fn cookie_only_backup_and_audit_exports_are_forbidden() {
    let app = build_router(test_state().await);
    let cookie = login_and_get_session_cookie(&app).await;
    let csrf = csrf_token_from_cookie(&cookie);
    for uri in ["/backup/export.json", "/audit/export.csv"] {
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("denied export");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        let body = String::from_utf8(
            denied
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .expect("utf8");
        assert!(!body.contains("schema_version"));
        assert!(!body.contains("created_at,actor_username,action"));
    }
    let backup = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/backup/export.json?csrf_token={csrf}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("backup");
    assert_eq!(backup.status(), StatusCode::OK);
    let backup_body = String::from_utf8(
        backup
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    assert!(backup_body.contains("schema_version"));
    let audit = app
        .oneshot(
            Request::builder()
                .uri(format!("/audit/export.csv?csrf_token={csrf}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("audit");
    assert_eq!(audit.status(), StatusCode::OK);
    let csv = String::from_utf8(
        audit
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .expect("utf8");
    assert!(csv.contains("created_at,actor_username,action"));
}

#[tokio::test]
async fn html_responses_include_nosniff_and_clickjacking_headers() {
    let app = build_router(test_state().await);
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("login");
    assert_eq!(login.status(), StatusCode::OK);
    assert!(has_clickjacking_and_nosniff(login.headers()));
    let cookie = login_and_get_session_cookie(&app).await;
    let home = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("home");
    assert_eq!(home.status(), StatusCode::OK);
    assert!(has_clickjacking_and_nosniff(home.headers()));
}
