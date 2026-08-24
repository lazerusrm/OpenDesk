mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use tower::ServiceExt;
use uuid::Uuid;

async fn grant_admin_visibility(state: &opendesk::AppState, device_uuids: &[Uuid]) {
    let admin = opendesk::repository::users::find_user_by_username(&state.db, "admin")
        .await
        .expect("lookup admin")
        .expect("admin");
    opendesk::repository::device_visibility::replace_user_device_visibility_grants(
        &state.db,
        admin.user_uuid,
        device_uuids,
    )
    .await
    .expect("grant device visibility");
}

async fn get_devices_html(uri: &str, session_cookie: &str, app: axum::Router) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("devices list");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(body.to_vec()).expect("utf8")
}

#[tokio::test]
async fn default_device_list_hides_archived_devices() {
    let state = test_state().await;
    let active = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Active List Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create active device");
    let archived = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Archived List Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create archived device");
    opendesk::repository::devices::set_device_archived(&state.db, archived.device_uuid, true)
        .await
        .expect("archive device");
    grant_admin_visibility(&state, &[active.device_uuid, archived.device_uuid]).await;

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let html = get_devices_html("/devices", &session_cookie, app).await;
    assert!(html.contains("Active List Device"));
    assert!(!html.contains("Archived List Device"));
    assert!(html.contains(r#"<option value="0" selected>Hide archived</option>"#));
    assert!(!html.contains("No devices yet"));
    assert!(!html.contains("No devices match this filter."));
}

#[tokio::test]
async fn empty_device_list_shows_empty_state() {
    let state = test_state().await;
    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let html = get_devices_html("/devices", &session_cookie, app).await;
    assert!(html.contains("No devices yet"));
    assert!(html.contains(r#"href="/devices/new""#));
    assert!(html.contains(r#"href="/enrollment-tokens""#));
    assert!(html.contains(r#"href="/deployment""#));
    assert!(html.contains("New device"));
    assert!(html.contains("Enrollment tokens"));
    assert!(html.contains("Deployment"));
    assert!(!html.contains("No devices match this filter."));
    assert!(!html.contains("<th>Status</th>"));
}

#[tokio::test]
async fn unmatched_filter_shows_no_match_state() {
    let state = test_state().await;
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Filter Workstation".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create device");
    grant_admin_visibility(&state, &[device.device_uuid]).await;

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let html = get_devices_html("/devices?term=nomatch", &session_cookie, app).await;
    assert!(html.contains("No devices match this filter."));
    assert!(!html.contains("No devices yet"));
    assert!(!html.contains("Filter Workstation"));
}

#[tokio::test]
async fn archived_filter_can_show_archived_and_all_devices() {
    let state = test_state().await;
    let active = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Active Filter Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create active device");
    let archived = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Archived Filter Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create archived device");
    opendesk::repository::devices::set_device_archived(&state.db, archived.device_uuid, true)
        .await
        .expect("archive device");
    grant_admin_visibility(&state, &[active.device_uuid, archived.device_uuid]).await;

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let archived_only = get_devices_html("/devices?archived=1", &session_cookie, app.clone()).await;
    assert!(archived_only.contains("Archived Filter Device"));
    assert!(!archived_only.contains("Active Filter Device"));
    assert!(archived_only.contains(r#"<option value="1" selected>Archived only</option>"#));

    let all_devices = get_devices_html("/devices?archived=all", &session_cookie, app).await;
    assert!(all_devices.contains("Active Filter Device"));
    assert!(all_devices.contains("Archived Filter Device"));
    assert!(all_devices.contains(r#"<option value="all" selected>All</option>"#));
}

#[tokio::test]
async fn status_column_uses_recently_seen_not_session_claims() {
    let state = test_state().await;
    let stale = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Stale Checkin Device".to_string(),
            rustdesk_id: Some("111222333".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create stale device");
    let recent = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Recent Checkin Device".to_string(),
            rustdesk_id: Some("444555666".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create recent device");
    opendesk::repository::devices::touch_device_checkin(
        &state.db,
        recent.device_uuid,
        &opendesk::domain::device::DeviceDraft {
            alias: recent.alias.clone(),
            rustdesk_id: recent.rustdesk_id.clone(),
            ..Default::default()
        },
    )
    .await
    .expect("touch recent check-in");
    grant_admin_visibility(&state, &[stale.device_uuid, recent.device_uuid]).await;

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let html = get_devices_html("/devices", &session_cookie, app.clone()).await;
    assert!(html.contains("<th>Status</th>"));
    assert!(html.contains(">Not recently seen<"));
    assert!(html.contains(r#"class="pill pill-ok""#));
    assert!(html.contains(
        "Based on enrollment check-in or client heartbeat last-seen. Not proof of an active RustDesk session."
    ));
    assert!(!html.to_ascii_lowercase().contains("online"));
    assert!(!html.contains("live session"));
    assert!(html.contains(r#"data-copy-text="111222333""#));
    assert!(html.contains(r#"data-copy-text="444555666""#));

    let recent_only = get_devices_html("/devices?seen=recent", &session_cookie, app).await;
    assert!(recent_only.contains("Recent Checkin Device"));
    assert!(!recent_only.contains("Stale Checkin Device"));
    assert!(recent_only.contains(r#"<option value="recent" selected>Recently seen</option>"#));
}
