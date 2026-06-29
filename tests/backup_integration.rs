mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::backup::{parse_backup_json, render_backup_json, BACKUP_SCHEMA_VERSION};
use opendesk::repository::backup::{export_backup_document, restore_backup_document};
use tower::ServiceExt;

#[tokio::test]
async fn backup_export_requires_auth() {
    let state = test_state().await;
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/backup/export.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("backup export");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn backup_export_and_restore_round_trip() {
    let source = test_state().await;
    let site = opendesk::repository::sites::create_site(
        &source.db,
        &opendesk::domain::site::SiteDraft {
            name: "Backup Site".to_string(),
        },
    )
    .await
    .expect("create site");
    let device = opendesk::repository::devices::create_device(
        &source.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Backup Device".to_string(),
            rustdesk_id: Some("424242424".to_string()),
            site_uuid: Some(site.site_uuid),
            notes: Some("restore me".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create device");

    let exported = export_backup_document(&source.db)
        .await
        .expect("export backup");
    assert_eq!(exported.schema_version, BACKUP_SCHEMA_VERSION);
    let json = render_backup_json(&exported).expect("serialize backup");
    let parsed = parse_backup_json(&json).expect("parse backup");

    let target = test_state().await;
    restore_backup_document(&target.db, &parsed)
        .await
        .expect("restore backup");
    let restored = opendesk::repository::devices::find_device_by_uuid(&target.db, device.device_uuid)
        .await
        .expect("lookup restored device")
        .expect("restored device");
    assert_eq!(restored.alias, "Backup Device");
    assert_eq!(restored.rustdesk_id.as_deref(), Some("424242424"));
    assert_eq!(restored.notes.as_deref(), Some("restore me"));
    assert_eq!(restored.site_uuid, Some(site.site_uuid));
}

#[tokio::test]
async fn backup_export_json_endpoint_returns_schema_version() {
    let state = test_state().await;
    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/backup/export.json")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("backup export");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("\"schema_version\": 1"));
    assert!(json.contains("\"excludes_sessions\": true"));
}