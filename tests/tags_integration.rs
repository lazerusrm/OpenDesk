mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use common::{login_and_get_session_cookie, test_state};
use opendesk::build_router;
use tower::ServiceExt;

#[tokio::test]
async fn tag_create_and_device_assignment_persist() {
    let state = test_state().await;
    let app = build_router(state.clone());
    let session_cookie = login_and_get_session_cookie(&app).await;

    let create_tag = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tags")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie.clone())
                .body(Body::from("name=Production"))
                .unwrap(),
        )
        .await
        .expect("create tag");
    assert_eq!(create_tag.status(), StatusCode::SEE_OTHER);

    let tags = opendesk::repository::tags::list_tags(&state.db)
        .await
        .expect("list tags");
    let tag = tags
        .iter()
        .find(|tag| tag.name == "Production")
        .expect("created tag");

    let create_device = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/devices")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie.clone())
                .body(Body::from(format!(
                    "alias=Tagged+Workstation&tag_uuids={}",
                    tag.tag_uuid
                )))
                .unwrap(),
        )
        .await
        .expect("create device");
    assert_eq!(create_device.status(), StatusCode::SEE_OTHER);

    let devices = opendesk::repository::devices::list_devices(&state.db)
        .await
        .expect("list devices");
    let device = devices
        .iter()
        .find(|device| device.alias == "Tagged Workstation")
        .expect("created device");
    let assigned = opendesk::repository::tags::list_tag_uuids_for_device(
        &state.db,
        device.device_uuid,
    )
    .await
    .expect("list device tags");
    assert_eq!(assigned, vec![tag.tag_uuid]);

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/devices")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("devices list");
    assert_eq!(list_response.status(), StatusCode::OK);
    let body = list_response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Production"));
    assert!(html.contains("Tagged Workstation"));
}

#[tokio::test]
async fn device_search_matches_tag_name() {
    let state = test_state().await;
    let tag = opendesk::repository::tags::create_tag(
        &state.db,
        &opendesk::domain::tag::TagDraft {
            name: "Warehouse".to_string(),
        },
    )
    .await
    .expect("create tag");
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Scanner PC".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create device");
    opendesk::repository::tags::set_device_tags(&state.db, device.device_uuid, &[tag.tag_uuid])
        .await
        .expect("assign tag");

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/devices?term=warehouse")
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("search devices");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Scanner PC"));
    assert!(html.contains("Warehouse"));
}

#[tokio::test]
async fn device_update_clears_tags_when_none_selected() {
    let state = test_state().await;
    let tag = opendesk::repository::tags::create_tag(
        &state.db,
        &opendesk::domain::tag::TagDraft {
            name: "Temporary".to_string(),
        },
    )
    .await
    .expect("create tag");
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Clear Tags Device".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create device");
    opendesk::repository::tags::set_device_tags(&state.db, device.device_uuid, &[tag.tag_uuid])
        .await
        .expect("assign tag");

    let app = build_router(state.clone());
    let session_cookie = login_and_get_session_cookie(&app).await;
    let update = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/devices/{}", device.device_uuid))
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie)
                .body(Body::from("alias=Clear+Tags+Device&site_uuid="))
                .unwrap(),
        )
        .await
        .expect("update device");
    assert_eq!(update.status(), StatusCode::SEE_OTHER);

    let assigned = opendesk::repository::tags::list_tag_uuids_for_device(
        &state.db,
        device.device_uuid,
    )
    .await
    .expect("list device tags");
    assert!(assigned.is_empty());
}