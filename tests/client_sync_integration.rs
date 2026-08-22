mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use opendesk::{build_router, domain::device::DeviceDraft};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn post_json_with_token(
    app: &axum::Router,
    uri: &str,
    body: Value,
    token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn get(app: &axum::Router, uri: &str, token: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

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

async fn login(app: &axum::Router, username: &str, password: &str, id: &str) -> String {
    let response = post_json(
        app,
        "/api/login",
        json!({
            "username": username,
            "password": password,
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
async fn client_sync_returns_only_explicitly_scoped_groups_users_and_peers() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let operator =
        opendesk::repository::users::create_user(&db, "operator", "password", "operator")
            .await
            .expect("operator");
    let peer_user = opendesk::repository::users::create_user(&db, "peer", "password", "read_only")
        .await
        .expect("peer user");
    opendesk::repository::users::create_user(&db, "unrelated", "password", "operator")
        .await
        .expect("unrelated user");
    let visible = opendesk::repository::devices::create_device(
        &db,
        &DeviceDraft {
            rustdesk_id: Some("200001".into()),
            alias: "Visible".into(),
            hostname: Some("visible-host".into()),
            os_family: Some("Linux".into()),
            os_version: Some("Test".into()),
            owner: Some("operator".into()),
            notes: Some("visible note".into()),
            ..Default::default()
        },
    )
    .await
    .expect("visible device");
    opendesk::repository::devices::create_device(
        &db,
        &DeviceDraft {
            rustdesk_id: Some("200002".into()),
            alias: "Hidden".into(),
            ..Default::default()
        },
    )
    .await
    .expect("hidden device");
    let group_uuid = Uuid::new_v4();
    sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, 'Scoped Team')")
        .bind(group_uuid.to_string())
        .execute(&db)
        .await
        .expect("group");
    for user_uuid in [operator.user_uuid, peer_user.user_uuid] {
        sqlx::query(
            "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
        )
        .bind(group_uuid.to_string())
        .bind(user_uuid.to_string())
        .execute(&db)
        .await
        .expect("membership");
    }
    sqlx::query(
        "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)",
    )
    .bind(group_uuid.to_string())
    .bind(visible.device_uuid.to_string())
    .execute(&db)
    .await
    .expect("visibility");
    let other_group_uuid = Uuid::new_v4();
    sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, 'Other Team')")
        .bind(other_group_uuid.to_string())
        .execute(&db)
        .await
        .expect("other group");
    sqlx::query(
        "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)",
    )
    .bind(other_group_uuid.to_string())
    .bind(visible.device_uuid.to_string())
    .execute(&db)
    .await
    .expect("other visibility");
    let book = opendesk::repository::address_books::create_personal_address_book(
        &db,
        operator.user_uuid,
        "Migrated address book",
    )
    .await
    .expect("address book");
    opendesk::repository::address_books::create_address_book_entry(
        &db,
        operator.user_uuid,
        book.address_book_uuid,
        visible.device_uuid,
        "Saved visible",
        Some("saved note".into()),
        0,
    )
    .await
    .expect("address book entry");
    sqlx::query(
        "INSERT INTO address_book_tags (address_book_uuid, name, color) VALUES (?, 'Critical', 7)",
    )
    .bind(book.address_book_uuid.to_string())
    .execute(&db)
    .await
    .expect("address book tag");
    let _owned_shared_book = opendesk::repository::address_books::create_shared_address_book(
        &db,
        operator.user_uuid,
        "Owned shared book",
    )
    .await
    .expect("owned shared book");
    let shared_book = opendesk::repository::address_books::create_shared_address_book(
        &db,
        peer_user.user_uuid,
        "Shared book",
    )
    .await
    .expect("shared book");
    sqlx::query(
        "INSERT INTO address_book_access_rules
         (address_book_uuid, principal_type, principal_uuid, permission)
         VALUES (?, 'group', ?, 'write')",
    )
    .bind(shared_book.address_book_uuid.to_string())
    .bind(group_uuid.to_string())
    .execute(&db)
    .await
    .expect("share rule");
    opendesk::repository::address_books::create_address_book_entry(
        &db,
        peer_user.user_uuid,
        shared_book.address_book_uuid,
        visible.device_uuid,
        "Shared visible",
        None,
        0,
    )
    .await
    .expect("shared entry");
    let app = build_router(state);
    let token = login(&app, "operator", "password", "300001").await;

    let groups = get(
        &app,
        "/api/device-group/accessible?current=1&pageSize=100",
        Some(&token),
    )
    .await;
    assert_eq!(groups.status(), StatusCode::OK);
    let groups = body_json(groups).await;
    assert_eq!(groups["total"], 1);
    assert_eq!(groups["data"][0]["name"], "Scoped Team");

    let users = get(
        &app,
        "/api/users?current=1&pageSize=100&accessible=&status=1",
        Some(&token),
    )
    .await;
    assert_eq!(users.status(), StatusCode::OK);
    let users = body_json(users).await;
    let names: Vec<&str> = users["data"]
        .as_array()
        .expect("users")
        .iter()
        .map(|user| user["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["operator", "peer"]);

    let peers = get(
        &app,
        "/api/peers?current=1&pageSize=100&accessible=&status=1",
        Some(&token),
    )
    .await;
    assert_eq!(peers.status(), StatusCode::OK);
    let peers = body_json(peers).await;
    assert_eq!(peers["total"], 1);
    assert_eq!(peers["data"][0]["id"], "200001");
    assert_eq!(peers["data"][0]["info"]["device_name"], "visible-host");
    assert_eq!(peers["data"][0]["note"], "visible note");
    assert_eq!(peers["data"][0]["user_name"], "operator");
    assert_eq!(peers["data"][0]["user"], operator.user_uuid.to_string());
    assert_eq!(peers["data"][0]["device_group_name"], "Scoped Team");

    let settings = post_json_with_token(&app, "/api/ab/settings", json!({}), &token).await;
    assert_eq!(settings.status(), StatusCode::OK);
    assert_eq!(body_json(settings).await["max_peer_one_ab"], 10000);
    let shared_profiles = post_json_with_token(
        &app,
        "/api/ab/shared/profiles?current=1&pageSize=100",
        json!({}),
        &token,
    )
    .await;
    assert_eq!(shared_profiles.status(), StatusCode::OK);
    let shared_profiles = body_json(shared_profiles).await;
    assert_eq!(shared_profiles["total"], 2);
    let profiles = shared_profiles["data"].as_array().expect("profiles");
    let owned = profiles
        .iter()
        .find(|profile| profile["name"] == "Owned shared book")
        .expect("owned shared profile");
    assert_eq!(owned["owner"], "operator");
    assert_eq!(owned["rule"], 3);
    let shared = profiles
        .iter()
        .find(|profile| profile["name"] == "Shared book")
        .expect("shared profile");
    assert_eq!(shared["owner"], "peer");
    assert_eq!(shared["rule"], 2);
    let shared_peers = post_json_with_token(
        &app,
        &format!(
            "/api/ab/peers?current=1&pageSize=100&ab={}",
            shared_book.address_book_uuid
        ),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(shared_peers.status(), StatusCode::OK);
    assert_eq!(
        body_json(shared_peers).await["data"][0]["alias"],
        "Shared visible"
    );
    let personal = post_json_with_token(&app, "/api/ab/personal", json!({}), &token).await;
    assert_eq!(personal.status(), StatusCode::OK);
    assert_eq!(
        body_json(personal).await["guid"],
        book.address_book_uuid.to_string()
    );
    let ab_peers = post_json_with_token(
        &app,
        &format!(
            "/api/ab/peers?current=1&pageSize=100&ab={}",
            book.address_book_uuid
        ),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(ab_peers.status(), StatusCode::OK);
    let ab_peers = body_json(ab_peers).await;
    assert_eq!(ab_peers["total"], 1);
    assert_eq!(ab_peers["data"][0]["id"], "200001");
    assert_eq!(ab_peers["data"][0]["alias"], "Saved visible");
    assert_eq!(ab_peers["data"][0]["note"], "saved note");
    assert_eq!(ab_peers["data"][0]["password"], "");
    assert_eq!(ab_peers["data"][0]["hash"], "");
    let tags = post_json_with_token(
        &app,
        &format!("/api/ab/tags/{}", book.address_book_uuid),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(tags.status(), StatusCode::OK);
    assert_eq!(
        body_json(tags).await,
        json!([{"name": "Critical", "color": 7}])
    );
}

#[tokio::test]
async fn client_sync_requires_valid_bearer_and_canonical_queries() {
    let app = build_router(common::test_state().await);
    assert_eq!(
        get(
            &app,
            "/api/peers?current=1&pageSize=100&accessible=&status=1",
            None,
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let token = login(&app, "admin", "test-password", "300002").await;
    assert_eq!(
        get(
            &app,
            "/api/peers?current=0&pageSize=100&accessible=&status=1",
            Some(&token),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
}
