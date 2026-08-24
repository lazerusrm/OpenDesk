mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::test_state;
use opendesk::build_router;
use opendesk::repository::audit_events::list_audit_events;
use serde_json::json;
use tower::ServiceExt;

async fn audit_event_count(db: &sqlx::SqlitePool) -> i64 {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events")
        .fetch_one(db)
        .await
        .expect("count audit_events");
    count
}

#[tokio::test]
async fn unknown_checkin_token_does_not_insert_audit_event() {
    let state = test_state().await;
    let db = state.db.clone();
    let before = audit_event_count(&db).await;
    let app = build_router(state);
    for index in 0..8 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/enrollments/check-in")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "enrollment_token": format!("garbage-token-{index}"),
                            "hostname": "attacker-host"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    assert_eq!(audit_event_count(&db).await, before);
    let events = list_audit_events(&db, 100).await.expect("audit");
    assert!(!events.iter().any(|event| event.action == "endpoint_checkin"));
}

#[tokio::test]
async fn revoked_checkin_token_inserts_failure_audit() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "revoked-checkin",
        None,
        None,
        None,
    )
    .await
    .expect("create token");
    opendesk::repository::enrollment_tokens::revoke_enrollment_token(
        &state.db,
        created.record.enrollment_token_uuid,
    )
    .await
    .expect("revoke");
    let db = state.db.clone();
    let before = audit_event_count(&db).await;
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
                        "hostname": "revoked-host"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(audit_event_count(&db).await, before + 1);
    let events = list_audit_events(&db, 20).await.expect("audit");
    let failure = events
        .iter()
        .find(|event| event.action == "endpoint_checkin" && event.outcome == "failure")
        .expect("failure audit");
    let detail = failure.detail_json.as_deref().unwrap_or("");
    assert!(detail.contains("token_invalid_or_revoked"));
    assert!(!detail.contains(&created.token_value));
}
