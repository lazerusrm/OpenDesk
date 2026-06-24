mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device_csv::DEVICE_CSV_HEADER;
use tower::ServiceExt;

#[tokio::test]
async fn devices_csv_export_requires_auth() {
    let state = test_state().await;
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/devices/export.csv")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("csv export");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn devices_csv_export_includes_expected_fields() {
    let state = test_state().await;
    let site = opendesk::repository::sites::create_site(
        &state.db,
        &opendesk::domain::site::SiteDraft {
            name: "Export Lab".to_string(),
        },
    )
    .await
    .expect("create site");
    let tag = opendesk::repository::tags::create_tag(
        &state.db,
        &opendesk::domain::tag::TagDraft {
            name: "Fleet".to_string(),
        },
    )
    .await
    .expect("create tag");
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "CSV Export Device".to_string(),
            rustdesk_id: Some("555444333".to_string()),
            hostname: Some("export-host".to_string()),
            notes: Some("export note".to_string()),
            site_uuid: Some(site.site_uuid),
            ..Default::default()
        },
    )
    .await
    .expect("create device");
    opendesk::repository::tags::set_device_tags(&state.db, device.device_uuid, &[tag.tag_uuid])
        .await
        .expect("assign tag");
    opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Archived Export Device".to_string(),
            rustdesk_id: Some("111222333".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create archived");
    let archived = opendesk::repository::devices::list_devices(&state.db)
        .await
        .expect("list")
        .into_iter()
        .find(|entry| entry.alias == "Archived Export Device")
        .expect("archived device");
    opendesk::repository::devices::set_device_archived(&state.db, archived.device_uuid, true)
        .await
        .expect("archive");

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/devices/export.csv")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("csv export");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/csv; charset=utf-8")
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let csv = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(csv.starts_with(DEVICE_CSV_HEADER));
    assert!(csv.contains("CSV Export Device"));
    assert!(csv.contains("555444333"));
    assert!(csv.contains("export-host"));
    assert!(csv.contains("Export Lab"));
    assert!(csv.contains("Fleet"));
    assert!(csv.contains("export note"));
    assert!(!csv.contains("Archived Export Device"));
}