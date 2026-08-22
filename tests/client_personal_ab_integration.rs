mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::domain::role::Role;
use opendesk::repository::{access_groups, device_visibility, devices, users};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json")
}

async fn client_login(app: &axum::Router, username: &str, password: &str, id: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "username": username,
                        "password": password,
                        "id": id,
                        "uuid": format!("{id}-uuid"),
                        "type": "account",
                        "deviceInfo": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("login");
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await["access_token"]
        .as_str()
        .expect("token")
        .to_string()
}

async fn post_json(
    app: &axum::Router,
    uri: &str,
    token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .expect("response")
}

#[tokio::test]
async fn official_client_personal_ab_auto_creates_and_lists_visible_devices() {
    let state = common::test_state().await;
    let operator = users::create_user(&state.db, "ab-user", "password", Role::OPERATOR)
        .await
        .expect("operator");
    let visible = devices::create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("600001".into()),
            alias: "Shop tablet".into(),
            notes: Some("visible note".into()),
            ..Default::default()
        },
    )
    .await
    .expect("visible");
    devices::create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("600002".into()),
            alias: "Hidden kiosk".into(),
            ..Default::default()
        },
    )
    .await
    .expect("hidden");
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        operator.user_uuid,
        &[visible.device_uuid],
    )
    .await
    .expect("grant");
    let app = build_router(state);
    let token = client_login(&app, "ab-user", "password", "600010").await;

    let personal = post_json(&app, "/api/ab/personal", &token).await;
    assert_eq!(personal.status(), StatusCode::OK);
    let guid = body_json(personal).await["guid"]
        .as_str()
        .expect("guid")
        .to_string();
    let again = post_json(&app, "/api/ab/personal", &token).await;
    assert_eq!(again.status(), StatusCode::OK);
    assert_eq!(body_json(again).await["guid"], guid);

    let peers = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/peers?current=1&pageSize=100&accessible=&status=1")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("peers");
    assert_eq!(peers.status(), StatusCode::OK);
    let peers = body_json(peers).await;
    assert_eq!(peers["total"], 1);
    assert_eq!(peers["data"][0]["id"], "600001");

    let ab_peers = post_json(
        &app,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        &token,
    )
    .await;
    assert_eq!(ab_peers.status(), StatusCode::OK);
    let ab_peers = body_json(ab_peers).await;
    assert_eq!(ab_peers["total"], 1);
    assert_eq!(ab_peers["data"][0]["id"], "600001");
    assert_eq!(ab_peers["data"][0]["alias"], "Shop tablet");
    assert_eq!(ab_peers["data"][0]["note"], "visible note");
    assert_eq!(ab_peers["data"][0]["password"], "");
}

#[tokio::test]
async fn admin_personal_ab_does_not_auto_fill_ungranted_devices() {
    let state = common::test_state().await;
    devices::create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("800001".into()),
            alias: "Ungranted".into(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let app = build_router(state);
    let token = client_login(&app, "admin", "test-password", "800010").await;
    let personal = post_json(&app, "/api/ab/personal", &token).await;
    assert_eq!(personal.status(), StatusCode::OK);
    let guid = body_json(personal).await["guid"]
        .as_str()
        .expect("guid")
        .to_string();
    let ab_peers = post_json(
        &app,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        &token,
    )
    .await;
    assert_eq!(ab_peers.status(), StatusCode::OK);
    assert_eq!(body_json(ab_peers).await["total"], 0);
    let peers = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/peers?current=1&pageSize=100&accessible=&status=1")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("peers");
    assert_eq!(body_json(peers).await["total"], 1);
}

#[tokio::test]
async fn deleted_personal_peer_is_not_restored_on_next_personal_pull() {
    let state = common::test_state().await;
    let admin = users::find_user_by_username(&state.db, "admin")
        .await
        .expect("lookup")
        .expect("admin");
    let device = devices::create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("900001".into()),
            alias: "Keep hidden".into(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        admin.user_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("grant");
    let app = build_router(state);
    let token = client_login(&app, "admin", "test-password", "900010").await;
    let personal = post_json(&app, "/api/ab/personal", &token).await;
    let guid = body_json(personal).await["guid"]
        .as_str()
        .expect("guid")
        .to_string();
    let before = post_json(
        &app,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        &token,
    )
    .await;
    assert_eq!(body_json(before).await["total"], 1);
    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/ab/peer/{guid}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from("[\"900001\"]"))
                .unwrap(),
        )
        .await
        .expect("delete");
    assert_eq!(deleted.status(), StatusCode::OK);
    let _ = post_json(&app, "/api/ab/personal", &token).await;
    let after = post_json(
        &app,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        &token,
    )
    .await;
    assert_eq!(body_json(after).await["total"], 0);
}

#[tokio::test]
async fn group_access_grant_lists_outgoing_devices_in_client_ab() {
    let state = common::test_state().await;
    let operator = users::create_user(&state.db, "helpdesk", "password", Role::OPERATOR)
        .await
        .expect("operator");
    let visible = devices::create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("700001".into()),
            alias: "Shop PC".into(),
            ..Default::default()
        },
    )
    .await
    .expect("visible");
    devices::create_device(
        &state.db,
        &DeviceDraft {
            rustdesk_id: Some("700002".into()),
            alias: "Other PC".into(),
            ..Default::default()
        },
    )
    .await
    .expect("hidden");
    let incoming = access_groups::create_access_group(&state.db, "Helpdesk")
        .await
        .expect("incoming");
    let outgoing = access_groups::create_access_group(&state.db, "Shop")
        .await
        .expect("outgoing");
    access_groups::replace_access_group_memberships(
        &state.db,
        incoming.access_group_uuid,
        &[operator.user_uuid],
    )
    .await
    .expect("membership");
    access_groups::replace_group_device_visibility_grants(
        &state.db,
        outgoing.access_group_uuid,
        &[visible.device_uuid],
    )
    .await
    .expect("outgoing devices");
    device_visibility::replace_outgoing_access_group_access_grants(
        &state.db,
        incoming.access_group_uuid,
        &[outgoing.access_group_uuid],
    )
    .await
    .expect("group access");
    let app = build_router(state);
    let token = client_login(&app, "helpdesk", "password", "700010").await;
    let personal = post_json(&app, "/api/ab/personal", &token).await;
    assert_eq!(personal.status(), StatusCode::OK);
    let guid = body_json(personal).await["guid"]
        .as_str()
        .expect("guid")
        .to_string();
    let ab_peers = post_json(
        &app,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        &token,
    )
    .await;
    assert_eq!(ab_peers.status(), StatusCode::OK);
    let ab_peers = body_json(ab_peers).await;
    assert_eq!(ab_peers["total"], 1);
    assert_eq!(ab_peers["data"][0]["id"], "700001");
    assert_eq!(ab_peers["data"][0]["alias"], "Shop PC");
}
