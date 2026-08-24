mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{form_with_csrf, login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::domain::role::Role;
use opendesk::repository::{access_groups, devices, users};
use tower::ServiceExt;

async fn html_for(app: &axum::Router, uri: &str, cookie: &str) -> (StatusCode, String) {
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
async fn access_groups_list_empty_state_explains_dashboard_visibility() {
    let state = test_state().await;
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    let (status, html) = html_for(&app, "/access-groups", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("No access groups yet."));
    assert!(html.contains("New group"));
    assert!(html.contains("Create group"));
    assert!(html.contains("href=\"#new-group\""));
    assert!(html.contains("href=\"/access-groups?view=users\""));
    assert!(html.contains("href=\"/access-groups?view=devices\""));
    assert!(html.contains("dashboard visibility only"));
    assert!(html.contains("do not control RustDesk sessions"));
    assert!(html.contains("not RustDesk session control"));
    assert!(!html.contains("session ACL"));
}

#[tokio::test]
async fn access_groups_list_shows_counts_query_tabs_and_known_views() {
    let state = test_state().await;
    let operator = users::create_user(
        &state.db,
        "shop-operator",
        "operator-password",
        Role::OPERATOR,
    )
    .await
    .expect("operator");
    let device = devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "Shop tablet".into(),
            rustdesk_id: Some("555111222".into()),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let group = access_groups::create_access_group(&state.db, "Shop operators")
        .await
        .expect("group");
    access_groups::replace_access_group_memberships(
        &state.db,
        group.access_group_uuid,
        &[operator.user_uuid],
    )
    .await
    .expect("memberships");
    access_groups::replace_group_device_visibility_grants(
        &state.db,
        group.access_group_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("visibility");
    let empty_group = access_groups::create_access_group(&state.db, "Empty group")
        .await
        .expect("empty group");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;

    let (status, html) = html_for(&app, "/access-groups", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Shop operators"));
    assert!(html.contains("Empty group"));
    assert!(html.contains("1 member"));
    assert!(html.contains("1 device"));
    assert!(html.contains("0 members"));
    assert!(html.contains("0 devices"));
    assert!(html.contains(&format!(
        "href=\"/access-groups/{}\"",
        group.access_group_uuid
    )));
    assert!(html.contains("Manage group"));
    assert!(html.contains("action=\"/access-groups\""));
    assert!(!html.contains("No access groups yet."));

    let (users_status, users_html) = html_for(&app, "/access-groups?view=users", &cookie).await;
    assert_eq!(users_status, StatusCode::OK);
    assert!(users_html.contains("User memberships"));
    assert!(users_html.contains("shop-operator"));
    assert!(users_html.contains("No members"));
    assert!(users_html.contains("do not control RustDesk sessions"));
    assert!(!users_html.contains("Visible devices"));
    assert!(!users_html.contains("session ACL"));

    let (devices_status, devices_html) =
        html_for(&app, "/access-groups?view=devices", &cookie).await;
    assert_eq!(devices_status, StatusCode::OK);
    assert!(devices_html.contains("Device visibility"));
    assert!(devices_html.contains("Shop tablet · 555111222"));
    assert!(devices_html.contains("No devices"));
    assert!(devices_html.contains("do not control RustDesk sessions"));
    assert!(!devices_html.contains("session ACL"));

    let (unknown_status, unknown_html) =
        html_for(&app, "/access-groups?view=sessions", &cookie).await;
    assert_eq!(unknown_status, StatusCode::OK);
    assert!(unknown_html.contains("All groups"));
    assert!(unknown_html.contains("1 member"));
    assert!(!unknown_html.contains("session ACL"));
    assert!(unknown_html.contains(&format!("/access-groups/{}", empty_group.access_group_uuid)));
}

#[tokio::test]
async fn access_group_detail_uses_checkbox_grid_and_shows_rustdesk_id() {
    let state = test_state().await;
    let operator = users::create_user(
        &state.db,
        "shop-operator",
        "operator-password",
        Role::OPERATOR,
    )
    .await
    .expect("operator");
    let device = devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "Shop tablet".into(),
            rustdesk_id: Some("555111222".into()),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "Unlabeled kiosk".into(),
            ..Default::default()
        },
    )
    .await
    .expect("unlabeled device");
    let group = access_groups::create_access_group(&state.db, "Shop operators")
        .await
        .expect("group");
    access_groups::replace_access_group_memberships(
        &state.db,
        group.access_group_uuid,
        &[operator.user_uuid],
    )
    .await
    .expect("memberships");
    access_groups::replace_group_device_visibility_grants(
        &state.db,
        group.access_group_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("visibility");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let detail_path = format!("/access-groups/{}", group.access_group_uuid);

    let (status, html) = html_for(&app, &detail_path, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Access group: Shop operators"));
    assert!(html.contains("1 member · 1 visible device"));
    assert!(html.contains("class=\"checkbox-row\""));
    assert!(html.contains("<fieldset>"));
    assert!(html.contains("<legend>Members</legend>"));
    assert!(html.contains("<legend>Visible devices</legend>"));
    assert!(html.contains("class=\"card-grid\""));
    assert!(html.contains("shop-operator"));
    assert!(html.contains("Shop tablet · 555111222"));
    assert!(html.contains("Unlabeled kiosk"));
    assert!(!html.contains("Unlabeled kiosk ·"));
    assert!(html.contains(&format!(
        "action=\"/access-groups/{}/memberships\"",
        group.access_group_uuid
    )));
    assert!(html.contains(&format!(
        "action=\"/access-groups/{}/device-visibility-grants\"",
        group.access_group_uuid
    )));
    assert!(html.contains(&format!(
        "action=\"/access-groups/{}/access-grants\"",
        group.access_group_uuid
    )));
    assert!(html.contains("<legend>Can see devices granted to</legend>"));
    assert!(html.contains("No other access groups available."));
    assert!(html.contains("dashboard and official-client data scope"));
    assert!(html.contains("dashboard visibility only"));
    assert!(html.contains("do not control RustDesk sessions"));
    assert!(!html.contains("Access group UUID"));
    assert!(!html.contains("<code>"));
    assert!(!html.contains("session ACL"));
}

#[tokio::test]
async fn access_group_detail_saves_outgoing_group_access() {
    let state = test_state().await;
    let incoming = access_groups::create_access_group(&state.db, "Helpdesk")
        .await
        .expect("incoming");
    let outgoing = access_groups::create_access_group(&state.db, "Shop")
        .await
        .expect("outgoing");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let detail_path = format!("/access-groups/{}", incoming.access_group_uuid);
    let (status, html) = html_for(&app, &detail_path, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Shop"));
    assert!(!html.contains(&format!("value=\"{}\" checked", outgoing.access_group_uuid)));
    let saved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/access-groups/{}/access-grants",
                    incoming.access_group_uuid
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    &format!("outgoing_access_group_uuid={}", outgoing.access_group_uuid),
                )))
                .unwrap(),
        )
        .await
        .expect("save group access");
    assert_eq!(saved.status(), StatusCode::SEE_OTHER);
    let (status, html) = html_for(&app, &detail_path, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(&format!("value=\"{}\" checked", outgoing.access_group_uuid)));
    assert!(html.contains("No other groups can see devices granted to this group."));
}
