mod common;

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use opendesk::build_router;
use serde_json::{json, Value};
use sqlx::Row;
use tower::ServiceExt;

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => {
            request = request
                .header("content-type", "application/json")
                .header("content-length", "0");
            Body::empty()
        }
    };
    app.clone()
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("response")
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec()
}

async fn body_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&body_bytes(response).await).expect("json")
}

async fn login(app: &axum::Router, username: &str, password: &str, id: &str) -> String {
    let response = request(
        app,
        Method::POST,
        "/api/login",
        None,
        Some(json!({
            "username": username,
            "password": password,
            "id": id,
            "uuid": format!("{id}-uuid"),
            "type": "account",
            "deviceInfo": {}
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await["access_token"]
        .as_str()
        .expect("token")
        .to_string()
}

async fn database_contains_secret(db: &sqlx::SqlitePool, secret: &str) -> bool {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(db)
    .await
    .expect("tables");
    for table in tables {
        let columns: Vec<String> =
            sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .fetch_all(db)
                .await
                .expect("columns");
        if columns.is_empty() {
            continue;
        }
        let predicates = columns
            .iter()
            .map(|column| format!("CAST(\"{column}\" AS TEXT) LIKE '%' || ? || '%'"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let sql = format!("SELECT COUNT(*) FROM \"{table}\" WHERE {predicates}");
        let mut query = sqlx::query_scalar::<_, i64>(&sql);
        for _ in &columns {
            query = query.bind(secret);
        }
        if query.fetch_one(db).await.expect("scan") > 0 {
            return true;
        }
    }
    false
}

#[tokio::test]
async fn address_book_password_requires_bearer_and_never_persists_secrets() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let token = login(&app, "admin", "test-password", "510001").await;
    let secret = "must-not-persist-ab-password";
    let hash = "must-not-persist-ab-hash";
    let user_hash_before: String =
        sqlx::query("SELECT password_hash FROM users WHERE username = ?")
            .bind("admin")
            .fetch_one(&db)
            .await
            .expect("user")
            .get("password_hash");
    let entries_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM address_book_entries")
        .fetch_one(&db)
        .await
        .expect("entries");

    let unauthorized = request(&app, Method::POST, "/api/ab/password", None, None).await;
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(unauthorized.headers()[CONTENT_TYPE], "application/json");
    assert_eq!(
        body_json(unauthorized).await,
        json!({"error": "Unauthorized"})
    );

    let put_password = request(
        &app,
        Method::PUT,
        "/api/ab/password",
        Some(&token),
        Some(json!({"id": "510001", "password": secret})),
    )
    .await;
    assert_eq!(put_password.status(), StatusCode::OK);
    assert!(body_bytes(put_password).await.is_empty());

    let post_hash = request(
        &app,
        Method::POST,
        "/api/ab/password",
        Some(&token),
        Some(json!({"id": "510001", "hash": hash})),
    )
    .await;
    assert_eq!(post_hash.status(), StatusCode::OK);
    assert!(body_bytes(post_hash).await.is_empty());

    let empty = request(&app, Method::POST, "/api/ab/password", Some(&token), None).await;
    assert_eq!(empty.status(), StatusCode::OK);
    assert!(body_bytes(empty).await.is_empty());

    let unknown = request(
        &app,
        Method::PUT,
        "/api/ab/password",
        Some(&token),
        Some(json!({"id": "510001", "password": secret, "unexpected": true})),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body_json(unknown).await,
        json!({"error": "Invalid request"})
    );

    let user_hash_after: String = sqlx::query("SELECT password_hash FROM users WHERE username = ?")
        .bind("admin")
        .fetch_one(&db)
        .await
        .expect("user")
        .get("password_hash");
    let entries_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM address_book_entries")
        .fetch_one(&db)
        .await
        .expect("entries");
    assert_eq!(user_hash_after, user_hash_before);
    assert_eq!(entries_after, entries_before);
    assert!(!database_contains_secret(&db, secret).await);
    assert!(!database_contains_secret(&db, hash).await);
}
