mod common;

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use opendesk::{build_router, domain::device::DeviceDraft};
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

async fn login(app: &axum::Router, username: &str, password: &str) -> String {
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
                        "id": "400002",
                        "uuid": "400002-uuid",
                        "type": "account",
                        "deviceInfo": {}
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await["access_token"]
        .as_str()
        .expect("token")
        .to_string()
}

async fn request(app: &axum::Router, uri: &str, token: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

#[tokio::test]
async fn official_1_4_9_legacy_get_returns_json_and_requires_bearer() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let user =
        opendesk::repository::users::create_user(&db, "legacy-client", "password", "operator")
            .await
            .expect("user");
    let device = opendesk::repository::devices::create_device(
        &db,
        &DeviceDraft {
            rustdesk_id: Some("400001".into()),
            alias: "Legacy peer".into(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    sqlx::query("INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)")
        .bind(user.user_uuid.to_string())
        .bind(device.device_uuid.to_string())
        .execute(&db)
        .await
        .expect("visibility");
    let book = opendesk::repository::address_books::create_personal_address_book(
        &db,
        user.user_uuid,
        "Legacy book",
    )
    .await
    .expect("book");
    opendesk::repository::address_books::create_address_book_entry(
        &db,
        user.user_uuid,
        book.address_book_uuid,
        device.device_uuid,
        "Saved legacy peer",
        None,
        0,
    )
    .await
    .expect("entry");
    let app = build_router(state);
    let token = login(&app, "legacy-client", "password").await;

    let unauthorized = request(&app, "/api/ab", None).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(unauthorized.headers()[CONTENT_TYPE], "application/json");
    assert_eq!(
        body_json(unauthorized).await,
        json!({"error": "Unauthorized"})
    );

    let response = request(&app, "/api/ab", Some(&token)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
    let response = body_json(response).await;
    assert_eq!(response["licensed_devices"], 10000);
    let data: Value =
        serde_json::from_str(response["data"].as_str().expect("data")).expect("nested JSON");
    assert_eq!(data["peers"][0]["id"], "400001");
    assert_eq!(data["peers"][0]["alias"], "Saved legacy peer");
    assert_eq!(data["tag_colors"], "{}");
}

#[tokio::test]
async fn official_1_4_9_empty_body_posts_return_json_unauthorized() {
    let app = build_router(common::test_state().await);
    for uri in [
        "/api/ab/personal",
        "/api/ab/settings",
        "/api/ab/shared/profiles?current=1&pageSize=100",
        "/api/ab/peers?current=1&pageSize=100&ab=00000000-0000-0000-0000-000000000000",
        "/api/ab/tags/00000000-0000-0000-0000-000000000000",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .header("content-length", "0")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
        assert_eq!(
            response.headers()[CONTENT_TYPE],
            "application/json",
            "{uri}"
        );
        assert_eq!(
            body_json(response).await,
            json!({"error": "Unauthorized"}),
            "{uri}"
        );
    }
}
