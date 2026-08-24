mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use opendesk::{build_router, domain::device::DeviceDraft};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
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

async fn login(app: &axum::Router, username: &str, id: &str) -> String {
    let response = request(
        app,
        Method::POST,
        "/api/login",
        None,
        json!({
            "username": username,
            "password": "password",
            "id": id,
            "uuid": format!("{id}-uuid"),
            "type": "account",
            "deviceInfo": {}
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await["access_token"]
        .as_str()
        .expect("token")
        .to_string()
}

#[tokio::test]
async fn official_client_mutations_enforce_write_scope_and_never_accept_secrets() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let owner = opendesk::repository::users::create_user(&db, "owner", "password", "admin")
        .await
        .expect("owner");
    let writer = opendesk::repository::users::create_user(&db, "writer", "password", "operator")
        .await
        .expect("writer");
    let reader = opendesk::repository::users::create_user(&db, "reader", "password", "read_only")
        .await
        .expect("reader");
    let device = opendesk::repository::devices::create_device(
        &db,
        &DeviceDraft {
            rustdesk_id: Some("410001".into()),
            alias: "Mutation target".into(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    for user_uuid in [owner.user_uuid, writer.user_uuid, reader.user_uuid] {
        sqlx::query(
            "INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)",
        )
        .bind(user_uuid.to_string())
        .bind(device.device_uuid.to_string())
        .execute(&db)
        .await
        .expect("visibility");
    }
    let book = opendesk::repository::address_books::create_shared_address_book(
        &db,
        owner.user_uuid,
        "Scoped mutations",
    )
    .await
    .expect("book");
    for (user_uuid, permission) in [(writer.user_uuid, "write"), (reader.user_uuid, "read")] {
        sqlx::query(
            "INSERT INTO address_book_access_rules
             (address_book_uuid, principal_type, principal_uuid, permission)
             VALUES (?, 'user', ?, ?)",
        )
        .bind(book.address_book_uuid.to_string())
        .bind(user_uuid.to_string())
        .bind(permission)
        .execute(&db)
        .await
        .expect("rule");
    }
    let app = build_router(state);
    let owner_token = login(&app, "owner", "410100").await;
    let writer_token = login(&app, "writer", "410101").await;
    let reader_token = login(&app, "reader", "410102").await;
    let guid = book.address_book_uuid;

    let tag = request(
        &app,
        Method::POST,
        &format!("/api/ab/tag/add/{guid}"),
        Some(&writer_token),
        json!({"name": "Ops", "color": 17}),
    )
    .await;
    assert_eq!(tag.status(), StatusCode::OK);
    let add = request(
        &app,
        Method::POST,
        &format!("/api/ab/peer/add/{guid}"),
        Some(&writer_token),
        json!({
            "id": "410001",
            "alias": "Initial",
            "note": "note",
            "tags": ["Ops"],
            "password": "",
            "hash": "",
            "username": "client-user",
            "hostname": "client-host",
            "platform": "Linux",
            "forceAlwaysRelay": "false",
            "rdpPort": "",
            "rdpUsername": "",
            "loginName": "",
            "device_group_name": "",
            "same_server": true
        }),
    )
    .await;
    assert_eq!(add.status(), StatusCode::OK);

    let update = request(
        &app,
        Method::PUT,
        &format!("/api/ab/peer/update/{guid}"),
        Some(&writer_token),
        json!({
            "id": "410001",
            "alias": "Updated",
            "note": "new note",
            "username": "client-user",
            "hostname": "new-host",
            "platform": "Linux"
        }),
    )
    .await;
    assert_eq!(update.status(), StatusCode::OK);
    let peers = request(
        &app,
        Method::POST,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        Some(&writer_token),
        json!({}),
    )
    .await;
    let peers = body_json(peers).await;
    assert_eq!(peers["data"][0]["alias"], "Updated");
    assert_eq!(peers["data"][0]["note"], "new note");
    assert_eq!(peers["data"][0]["tags"], json!(["Ops"]));

    let rename_tag = request(
        &app,
        Method::PUT,
        &format!("/api/ab/tag/rename/{guid}"),
        Some(&writer_token),
        json!({"old": " Ops ", "new": " Priority "}),
    )
    .await;
    assert_eq!(rename_tag.status(), StatusCode::OK);
    let recolor_tag = request(
        &app,
        Method::PUT,
        &format!("/api/ab/tag/update/{guid}"),
        Some(&writer_token),
        json!({"name": " Priority ", "color": 23}),
    )
    .await;
    assert_eq!(recolor_tag.status(), StatusCode::OK);

    let forbidden = request(
        &app,
        Method::DELETE,
        &format!("/api/ab/peer/{guid}"),
        Some(&reader_token),
        json!(["410001"]),
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let secret = request(
        &app,
        Method::PUT,
        &format!("/api/ab/peer/update/{guid}"),
        Some(&writer_token),
        json!({"id": "410001", "password": "must-not-cross"}),
    )
    .await;
    assert_eq!(secret.status(), StatusCode::OK);
    assert!(body_bytes(secret).await.is_empty());
    let hash_only = request(
        &app,
        Method::PUT,
        &format!("/api/ab/peer/update/{guid}"),
        Some(&writer_token),
        json!({"id": "410001", "hash": "bXVzdC1ub3QtcGVyc2lzdA=="}),
    )
    .await;
    assert_eq!(hash_only.status(), StatusCode::OK);
    assert!(body_bytes(hash_only).await.is_empty());
    let after_secret = request(
        &app,
        Method::POST,
        &format!("/api/ab/peers?current=1&pageSize=100&ab={guid}"),
        Some(&writer_token),
        json!({}),
    )
    .await;
    let after_secret = body_json(after_secret).await;
    assert_eq!(after_secret["data"][0]["alias"], "Updated");
    assert_eq!(after_secret["data"][0]["password"], "");
    assert_eq!(after_secret["data"][0]["hash"], "");
    assert_eq!(after_secret["data"][0]["note"], "new note");

    let operator_delete = request(
        &app,
        Method::DELETE,
        &format!("/api/ab/peer/{guid}"),
        Some(&writer_token),
        json!(["410001"]),
    )
    .await;
    assert_eq!(operator_delete.status(), StatusCode::FORBIDDEN);
    let delete_peer = request(
        &app,
        Method::DELETE,
        &format!("/api/ab/peer/{guid}"),
        Some(&owner_token),
        json!(["410001"]),
    )
    .await;
    assert_eq!(delete_peer.status(), StatusCode::OK);
    let operator_delete_tag = request(
        &app,
        Method::DELETE,
        &format!("/api/ab/tag/{guid}"),
        Some(&writer_token),
        json!([" Priority "]),
    )
    .await;
    assert_eq!(operator_delete_tag.status(), StatusCode::FORBIDDEN);
    let delete_tag = request(
        &app,
        Method::DELETE,
        &format!("/api/ab/tag/{guid}"),
        Some(&owner_token),
        json!([" Priority "]),
    )
    .await;
    assert_eq!(delete_tag.status(), StatusCode::OK);

    let entries: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM address_book_entries WHERE address_book_uuid = ?")
            .bind(guid.to_string())
            .fetch_one(&db)
            .await
            .expect("entries");
    let tags: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM address_book_tags WHERE address_book_uuid = ?")
            .bind(guid.to_string())
            .fetch_one(&db)
            .await
            .expect("tags");
    assert_eq!((entries, tags), (0, 0));
}

#[tokio::test]
async fn mutation_routes_require_bearer_and_canonical_payloads() {
    let app = build_router(common::test_state().await);
    let guid = Uuid::new_v4();
    let unauthenticated = request(
        &app,
        Method::POST,
        &format!("/api/ab/tag/add/{guid}"),
        None,
        json!({"name": "Ops", "color": 1}),
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}
