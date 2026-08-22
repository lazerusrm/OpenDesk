mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, login_and_get_session_cookie_as, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::role::Role;
use opendesk::repository::{address_books, users};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn get_html(app: &axum::Router, cookie: &str, uri: &str) -> String {
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
        .expect("page");
    assert_eq!(response.status(), StatusCode::OK);
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

async fn client_login(app: &axum::Router, username: &str, password: &str) -> String {
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
                        "id": "500001",
                        "uuid": "500001-uuid",
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
    let body: Value = serde_json::from_slice(
        &response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json");
    body["access_token"].as_str().expect("token").to_string()
}

#[tokio::test]
async fn shared_book_owner_can_grant_and_revoke_user_access() {
    let state = test_state().await;
    let operator = users::create_user(
        &state.db,
        "shop-operator",
        "operator-password",
        Role::OPERATOR,
    )
    .await
    .expect("operator");
    let admin = users::find_user_by_username(&state.db, "admin")
        .await
        .expect("lookup")
        .expect("admin");
    let book =
        address_books::create_shared_address_book(&state.db, admin.user_uuid, "Team book")
            .await
            .expect("shared book");
    let personal =
        address_books::create_personal_address_book(&state.db, admin.user_uuid, "Favorites")
            .await
            .expect("personal");
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let shared_path = format!("/address-books/{}", book.address_book_uuid);
    let html = get_html(&app, &cookie, &shared_path).await;
    assert!(html.contains("Share this address book"));
    assert!(html.contains("Share with a user"));
    assert!(html.contains("shop-operator"));
    assert!(html.contains("Not shared with any user or group yet."));

    let granted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("{shared_path}/access-rules"))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    &format!(
                        "principal_type=user&principal_uuid={}&permission=read",
                        operator.user_uuid
                    ),
                )))
                .unwrap(),
        )
        .await
        .expect("grant");
    assert_eq!(granted.status(), StatusCode::SEE_OTHER);
    let html = get_html(&app, &cookie, &shared_path).await;
    assert!(html.contains("shop-operator"));
    assert!(html.contains("Remove share"));
    assert!(html.contains(">Read<"));

    let operator_cookie =
        login_and_get_session_cookie_as(&app, "shop-operator", "operator-password").await;
    let operator_list = get_html(&app, &operator_cookie, "/address-books").await;
    assert!(operator_list.contains("Shared with you"));
    assert!(operator_list.contains("Team book"));
    assert!(operator_list.contains("Read"));

    let token = client_login(&app, "shop-operator", "operator-password").await;
    let profiles = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ab/shared/profiles?current=1&pageSize=100")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .expect("profiles");
    assert_eq!(profiles.status(), StatusCode::OK);
    let profiles: Value = serde_json::from_slice(
        &profiles
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes(),
    )
    .expect("json");
    assert_eq!(profiles["total"], 1);
    assert_eq!(profiles["data"][0]["name"], "Team book");
    assert_eq!(profiles["data"][0]["rule"], 1);

    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/address-books/{}/access-rules",
                    personal.address_book_uuid
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    &format!(
                        "principal_type=user&principal_uuid={}&permission=read",
                        operator.user_uuid
                    ),
                )))
                .unwrap(),
        )
        .await
        .expect("personal share");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let operator_revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "{shared_path}/access-rules/user/{}/delete",
                    operator.user_uuid
                ))
                .header("cookie", &operator_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&operator_cookie, "")))
                .unwrap(),
        )
        .await
        .expect("operator revoke");
    assert_eq!(operator_revoke.status(), StatusCode::FORBIDDEN);

    let revoked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "{shared_path}/access-rules/user/{}/delete",
                    operator.user_uuid
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&cookie, "")))
                .unwrap(),
        )
        .await
        .expect("revoke");
    assert_eq!(revoked.status(), StatusCode::SEE_OTHER);
    let operator_list = get_html(&app, &operator_cookie, "/address-books").await;
    assert!(operator_list.contains("Nothing has been shared with you yet."));
}
