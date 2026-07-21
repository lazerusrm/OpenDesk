mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, session_cookie_from_response, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::role::Role;
use opendesk::repository::audit_events::list_audit_events;
use opendesk::repository::users::create_user;
use tower::ServiceExt;

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

#[tokio::test]
async fn read_only_can_view_devices_but_cannot_mutate() {
    let state = test_state().await;
    create_user(&state.db, "viewer", "viewer-password", Role::READ_ONLY)
        .await
        .expect("create viewer");
    let app = build_router(state);
    let cookie = login_as(&app, "viewer", "viewer-password").await;

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/devices")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list");
    assert_eq!(list.status(), StatusCode::OK);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/devices")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "alias=Blocked+Device")))
                .unwrap(),
        )
        .await
        .expect("create");
    assert_eq!(create.status(), StatusCode::FORBIDDEN);

    let site = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sites")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "name=Blocked+Site")))
                .unwrap(),
        )
        .await
        .expect("site");
    assert_eq!(site.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn operator_can_mutate_devices_but_not_manage_users_or_restore() {
    let state = test_state().await;
    create_user(&state.db, "operator1", "operator-password", Role::OPERATOR)
        .await
        .expect("create operator");
    let app = build_router(state);
    let cookie = login_as(&app, "operator1", "operator-password").await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/devices")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "alias=Operator+Device")))
                .unwrap(),
        )
        .await
        .expect("create");
    assert_eq!(create.status(), StatusCode::SEE_OTHER);

    let users = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/users")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("users");
    assert_eq!(users.status(), StatusCode::FORBIDDEN);

    let restore = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backup")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "backup_json=%7B%7D&confirm=yes",
                )))
                .unwrap(),
        )
        .await
        .expect("restore");
    assert_eq!(restore.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_create_user_and_list_users() {
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

    let list = app
        .oneshot(
            Request::builder()
                .uri("/users")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list users");
    assert_eq!(list.status(), StatusCode::OK);
    let body = list.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("newops"));
    assert!(html.contains("operator"));
}

#[tokio::test]
async fn audit_log_lists_events_and_export_redacts_tokens() {
    let state = test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    // Create enrollment token (writes audit without raw secret in detail).
    let create_token = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/enrollment-tokens")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "label=fleet-a")))
                .unwrap(),
        )
        .await
        .expect("token");
    assert_eq!(create_token.status(), StatusCode::OK);
    let token_body = create_token.into_body().collect().await.unwrap().to_bytes();
    let token_html = String::from_utf8(token_body.to_vec()).expect("utf8");
    // Extract plaintext token from one-time display for a failed check-in that redacts it.
    let raw_token = token_html
        .lines()
        .find(|line| line.contains("created_token") || line.contains("token"))
        .and_then(|_| {
            // The page shows created token value once; pull a long alphanumeric-ish string.
            token_html
                .split_whitespace()
                .find(|part| part.len() >= 24 && part.chars().all(|c| c.is_ascii_alphanumeric()))
                .map(|s| s.to_string())
        });

    // Force a check-in failure with a known secret to prove redaction in stored detail.
    let secret = "super-secret-enrollment-token-value-xyz";
    let fail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"enrollment_token":"{secret}","hostname":"bad-host"}}"#
                )))
                .unwrap(),
        )
        .await
        .expect("checkin fail");
    assert_eq!(fail.status(), StatusCode::UNAUTHORIZED);

    let events = list_audit_events(&db, 100).await.expect("list audit");
    let failure = events
        .iter()
        .find(|e| e.action == "endpoint_checkin" && e.outcome == "failure")
        .expect("failure audit event");
    let detail = failure.detail_json.as_deref().unwrap_or("");
    assert!(
        detail.contains("[redacted]"),
        "expected redacted detail, got {detail}"
    );
    assert!(
        !detail.contains(secret),
        "raw enrollment token must not appear in audit detail"
    );
    let _ = raw_token;

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/audit")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("audit page");
    assert_eq!(page.status(), StatusCode::OK);
    let page_body = page.into_body().collect().await.unwrap().to_bytes();
    let page_html = String::from_utf8(page_body.to_vec()).expect("utf8");
    assert!(page_html.contains("Audit Log"));
    assert!(
        page_html.contains("endpoint_checkin") || page_html.contains("enrollment_token_create")
    );
    assert!(!page_html.contains(secret));

    let export = app
        .oneshot(
            Request::builder()
                .uri("/audit/export.csv")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("export");
    assert_eq!(export.status(), StatusCode::OK);
    let export_body = export.into_body().collect().await.unwrap().to_bytes();
    let csv = String::from_utf8(export_body.to_vec()).expect("utf8");
    assert!(csv.contains("created_at,actor_username,action,"));
    assert!(!csv.contains(secret));
    assert!(csv.contains("endpoint_checkin") || csv.contains("enrollment_token_create"));
}

#[tokio::test]
async fn read_only_can_view_audit_but_not_export_devices_csv() {
    let state = test_state().await;
    create_user(&state.db, "reader", "reader-password", Role::READ_ONLY)
        .await
        .expect("create reader");
    let app = build_router(state);
    let cookie = login_as(&app, "reader", "reader-password").await;

    let audit = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/audit")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("audit");
    assert_eq!(audit.status(), StatusCode::OK);

    let csv = app
        .oneshot(
            Request::builder()
                .uri("/devices/export.csv")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("csv");
    assert_eq!(csv.status(), StatusCode::FORBIDDEN);
}
