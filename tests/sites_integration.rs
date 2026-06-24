mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use common::{login_and_get_session_cookie, test_state};
use opendesk::build_router;
use tower::ServiceExt;
#[tokio::test]
async fn site_create_and_device_assignment_persist() {
    let state = test_state().await;
    let app = build_router(state.clone());
    let session_cookie = login_and_get_session_cookie(&app).await;

    let create_site = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/sites")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie.clone())
                .body(Body::from("name=Main+Lab"))
                .unwrap(),
        )
        .await
        .expect("create site");
    assert_eq!(create_site.status(), StatusCode::SEE_OTHER);

    let sites = opendesk::repository::sites::list_sites(&state.db)
        .await
        .expect("list sites");
    let site = sites
        .iter()
        .find(|site| site.name == "Main Lab")
        .expect("created site");

    let create_device = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/devices")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie.clone())
                .body(Body::from(format!(
                    "alias=Lab+Workstation&site_uuid={}",
                    site.site_uuid
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
        .find(|device| device.alias == "Lab Workstation")
        .expect("created device");
    assert_eq!(device.site_uuid, Some(site.site_uuid));

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
    assert!(html.contains("Main Lab"));
    assert!(html.contains("Lab Workstation"));
}

#[tokio::test]
async fn device_search_matches_site_name() {
    let state = test_state().await;
    let site = opendesk::repository::sites::create_site(
        &state.db,
        &opendesk::domain::site::SiteDraft {
            name: "Warehouse East".to_string(),
        },
    )
    .await
    .expect("create site");
    opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Forklift PC".to_string(),
            site_uuid: Some(site.site_uuid),
            ..Default::default()
        },
    )
    .await
    .expect("create device");

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
    assert!(html.contains("Forklift PC"));
    assert!(html.contains("Warehouse East"));
}