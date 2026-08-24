mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use opendesk::{build_router, domain::device::DeviceDraft};
use serde_json::{json, Value};
use sha2::Sha256;
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

fn internal_header() -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(&[9; 32]).expect("key");
    mac.update(b"opendesk-transport-introspection-v1");
    hex::encode(mac.finalize().into_bytes())
}

async fn post(
    app: &axum::Router,
    uri: &str,
    body: Value,
    internal_key: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(key) = internal_key {
        request = request.header("x-opendesk-transport-key", key);
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_string())).expect("request"))
        .await
        .expect("response")
}

#[tokio::test]
async fn transport_authorization_is_internal_and_default_deny() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let user = opendesk::repository::users::create_user(&db, "transport", "password", "operator")
        .await
        .expect("user");
    let visible = opendesk::repository::devices::create_device(
        &db,
        &DeviceDraft {
            rustdesk_id: Some("510001".into()),
            alias: "Visible transport peer".into(),
            ..Default::default()
        },
    )
    .await
    .expect("visible device");
    opendesk::repository::devices::create_device(
        &db,
        &DeviceDraft {
            rustdesk_id: Some("510002".into()),
            alias: "Hidden transport peer".into(),
            ..Default::default()
        },
    )
    .await
    .expect("hidden device");
    sqlx::query("INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)")
        .bind(user.user_uuid.to_string())
        .bind(visible.device_uuid.to_string())
        .execute(&db)
        .await
        .expect("visibility");
    let app = build_router(state);
    let login = post(
        &app,
        "/api/login",
        json!({
            "username": "transport",
            "password": "password",
            "id": "510100",
            "uuid": "transport-client-uuid",
            "type": "account",
            "deviceInfo": {}
        }),
        None,
    )
    .await;
    let token = body_json(login).await["access_token"]
        .as_str()
        .expect("token")
        .to_string();

    let missing_auth = post(
        &app,
        "/internal/transport/authorize",
        json!({"token": token, "rustdesk_id": "510001"}),
        None,
    )
    .await;
    assert_eq!(missing_auth.status(), StatusCode::UNAUTHORIZED);

    let key = internal_header();
    let allowed = post(
        &app,
        "/internal/transport/authorize",
        json!({"token": token, "rustdesk_id": "510001"}),
        Some(&key),
    )
    .await;
    assert_eq!(allowed.status(), StatusCode::OK);
    assert_eq!(body_json(allowed).await, json!({"allowed": true}));

    let hidden = post(
        &app,
        "/internal/transport/authorize",
        json!({"token": token, "rustdesk_id": "510002"}),
        Some(&key),
    )
    .await;
    assert_eq!(hidden.status(), StatusCode::OK);
    assert_eq!(body_json(hidden).await, json!({"allowed": false}));

    sqlx::query("UPDATE users SET activation_state = 'disabled' WHERE user_uuid = ?")
        .bind(user.user_uuid.to_string())
        .execute(&db)
        .await
        .expect("disable user");
    let disabled = post(
        &app,
        "/internal/transport/authorize",
        json!({"token": token, "rustdesk_id": "510001"}),
        Some(&key),
    )
    .await;
    assert_eq!(body_json(disabled).await, json!({"allowed": false}));
}
