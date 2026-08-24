mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    csrf_token_from_cookie, login_and_get_session_cookie, session_cookie_from_response, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::audit_event::AuditEventDraft;
use opendesk::domain::role::Role;
use opendesk::repository::audit_events::insert_audit_event;
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

async fn get_with_cookie(app: &axum::Router, cookie: &str, uri: &str) -> (StatusCode, String) {
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
        .expect("response");
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).expect("utf8"))
}

fn client_audit_draft(
    action: &str,
    object_type: &str,
    detail: Option<serde_json::Value>,
) -> AuditEventDraft {
    AuditEventDraft {
        actor_user_uuid: None,
        action: action.to_string(),
        object_type: object_type.to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "client".to_string(),
        detail,
    }
}

#[tokio::test]
async fn audit_kind_tabs_filter_single_table_without_fake_alarms() {
    let state = test_state().await;
    let db = state.db.clone();
    insert_audit_event(
        &db,
        &client_audit_draft(
            "client_connection_open",
            "device",
            Some(serde_json::json!({"peer":"reported-client"})),
        ),
    )
    .await
    .expect("connection event");
    insert_audit_event(
        &db,
        &client_audit_draft("client_file_transfer", "device", None),
    )
    .await
    .expect("file event");
    insert_audit_event(
        &db,
        &client_audit_draft("client_session_record", "session", None),
    )
    .await
    .expect("session event");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    let (status, all) = get_with_cookie(&app, &cookie, "/audit").await;
    assert_eq!(status, StatusCode::OK);
    assert!(all.contains("href=\"/audit?kind=console\""));
    assert!(all.contains("href=\"/audit?kind=connection\""));
    assert!(all.contains("client_connection_open"));
    assert!(all.contains("client_file_transfer"));
    assert!(all.contains("client_session_record"));
    assert!(all.contains("reported-client"));
    assert!(all.contains(
        "Client connection events are reports from official clients. They are not proof that OpenDesk enforced a session."
    ));

    let (status, console) = get_with_cookie(&app, &cookie, "/audit?kind=console").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!console.contains("client_connection_open"));
    assert!(!console.contains("client_file_transfer"));
    assert!(!console.contains("client_session_record"));

    let (status, connection) = get_with_cookie(&app, &cookie, "/audit?kind=connection").await;
    assert_eq!(status, StatusCode::OK);
    assert!(connection.contains("client_connection_open"));
    assert!(!connection.contains("client_file_transfer"));
    assert!(connection.contains("/audit/export.csv?kind=connection"));

    let (status, file) = get_with_cookie(&app, &cookie, "/audit?kind=file").await;
    assert_eq!(status, StatusCode::OK);
    assert!(file.contains("client_file_transfer"));
    assert!(!file.contains("client_connection_open"));

    let (status, alarm) = get_with_cookie(&app, &cookie, "/audit?kind=alarm").await;
    assert_eq!(status, StatusCode::OK);
    assert!(alarm.contains("OpenDesk does not ingest Pro alarm tables"));
    assert!(!alarm.contains("client_connection_open"));
    assert!(!alarm.contains("<table>"));

    let (status, _) = get_with_cookie(&app, &cookie, "/audit?kind=sessions").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, csv) = get_with_cookie(
        &app,
        &cookie,
        &format!(
            "/audit/export.csv?kind=connection&csrf_token={}",
            csrf_token_from_cookie(&cookie)
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(csv.contains("client_connection_open"));
    assert!(!csv.contains("client_file_transfer"));
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
    let (kinded_status, _) = get_with_cookie(&app, &cookie, "/audit?kind=console").await;
    assert_eq!(kinded_status, StatusCode::OK);

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
