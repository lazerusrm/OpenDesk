mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use opendesk::build_router;
use serde_json::{json, Value};
use tower::ServiceExt;

fn login_body(username: &str, password: &str, id: &str, uuid: &str) -> Value {
    json!({
        "username": username,
        "password": password,
        "id": id,
        "uuid": uuid,
        "autoLogin": true,
        "type": "account",
        "deviceInfo": {"os": "Linux", "type": "client", "name": "test"}
    })
}

async fn get(app: &axum::Router, uri: &str, token: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().method("GET").uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

async fn post_json(
    app: &axum::Router,
    uri: &str,
    body: Value,
    token: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_string())).expect("request"))
        .await
        .expect("response")
}

async fn response_json(response: axum::response::Response) -> Value {
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

#[tokio::test]
async fn login_options_returns_empty_oidc_array() {
    let state = common::test_state().await;
    let app = build_router(state);
    let response = get(&app, "/api/login-options", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    let options = body.as_array().expect("array");
    assert!(options.iter().any(|item| item == "common-oidc/[]"));

    let info = get(&app, "/api/oidc/info", None).await;
    assert_eq!(info.status(), StatusCode::OK);
    let info = response_json(info).await;
    assert_eq!(info, json!({}));

    let oidc_options = get(&app, "/api/oidc/login-options", None).await;
    assert_eq!(oidc_options.status(), StatusCode::OK);
    assert_eq!(response_json(oidc_options).await, json!([]));
}

#[tokio::test]
async fn current_user_get_requires_bearer_and_returns_admin() {
    let state = common::test_state().await;
    let app = build_router(state);
    let unauthenticated = get(&app, "/api/currentUser", None).await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let login = post_json(
        &app,
        "/api/login",
        login_body("admin", "test-password", "100001", "client-a"),
        None,
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let token = response_json(login).await["access_token"]
        .as_str()
        .expect("token")
        .to_string();

    let current = get(&app, "/api/currentUser", Some(&token)).await;
    assert_eq!(current.status(), StatusCode::OK);
    let current = response_json(current).await;
    assert_eq!(current["name"], "admin");
}
