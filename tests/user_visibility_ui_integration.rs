mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, session_cookie_from_response, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::device::DeviceDraft;
use opendesk::domain::role::Role;
use opendesk::repository::{access_groups, device_visibility, devices, users};
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
async fn operator_cannot_get_user_detail() {
    let state = test_state().await;
    let operator = users::create_user(&state.db, "operator", "operator-password", Role::OPERATOR)
        .await
        .expect("operator");
    let app = build_router(state);
    let cookie = login_as(&app, "operator", "operator-password").await;

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/users/{}", operator.user_uuid))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("user detail");
    assert_eq!(detail.status(), StatusCode::FORBIDDEN);

    let grants = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/users/{}/device-visibility-grants",
                    operator.user_uuid
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "device_uuid=00000000-0000-0000-0000-000000000000",
                )))
                .unwrap(),
        )
        .await
        .expect("user grants");
    assert_eq!(grants.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_direct_grants_scope_operator_device_list() {
    let state = test_state().await;
    let operator = users::create_user(&state.db, "operator", "operator-password", Role::OPERATOR)
        .await
        .expect("operator");
    let granted = devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "Granted device".into(),
            rustdesk_id: Some("123456789".into()),
            ..Default::default()
        },
    )
    .await
    .expect("granted device");
    let hidden = devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: "Hidden device".into(),
            rustdesk_id: Some("987654321".into()),
            ..Default::default()
        },
    )
    .await
    .expect("hidden device");
    let group = access_groups::create_access_group(&state.db, "Operators")
        .await
        .expect("group");
    access_groups::replace_access_group_memberships(
        &state.db,
        group.access_group_uuid,
        &[operator.user_uuid],
    )
    .await
    .expect("membership");
    let app = build_router(state.clone());
    let admin_cookie = login_and_get_session_cookie(&app).await;

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/users/{}", operator.user_uuid))
                .header("cookie", &admin_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("user detail");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_html = String::from_utf8(
        detail
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(detail_html.contains("operator"));
    assert!(detail_html.contains("Granted device"));
    assert!(detail_html.contains("Hidden device"));
    assert!(detail_html.contains("123456789"));
    assert!(detail_html.contains("Operators"));
    assert!(detail_html.contains(&format!("/access-groups/{}", group.access_group_uuid)));
    assert!(detail_html.contains(
        "These grants control OpenDesk dashboard/API device visibility only. They do not authorize or deny a RustDesk session."
    ));

    let save = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/users/{}/device-visibility-grants",
                    operator.user_uuid
                ))
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &admin_cookie,
                    &format!("device_uuid={}", granted.device_uuid),
                )))
                .unwrap(),
        )
        .await
        .expect("save grants");
    assert_eq!(save.status(), StatusCode::SEE_OTHER);

    let stored =
        device_visibility::list_user_device_visibility_grants(&state.db, operator.user_uuid)
            .await
            .expect("stored grants");
    assert_eq!(
        stored
            .into_iter()
            .map(|grant| grant.device_uuid)
            .collect::<Vec<_>>(),
        vec![granted.device_uuid]
    );

    let operator_cookie = login_as(&app, "operator", "operator-password").await;
    let devices_page = app
        .oneshot(
            Request::builder()
                .uri("/devices")
                .header("cookie", &operator_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("devices list");
    assert_eq!(devices_page.status(), StatusCode::OK);
    let devices_html = String::from_utf8(
        devices_page
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(devices_html.contains("Granted device"));
    assert!(!devices_html.contains("Hidden device"));
    let _ = hidden;
}
