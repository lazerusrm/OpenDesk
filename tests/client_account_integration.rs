mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bcrypt::hash;
use http_body_util::BodyExt;
use opendesk::{build_router, repository::users::find_user_by_username};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

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
async fn login_current_user_and_logout_bind_and_revoke_token() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let login = post_json(
        &app,
        "/api/login",
        login_body("admin", "test-password", "100001", "client-a"),
        None,
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let login = response_json(login).await;
    assert_eq!(login["type"], "access_token");
    assert_eq!(login["user"]["name"], "admin");
    assert_eq!(login["user"]["status"], 1);
    assert_eq!(login["user"]["is_admin"], true);
    let token = login["access_token"].as_str().expect("token");
    assert_eq!(token.len(), 64);

    let stored: (String,) = sqlx::query_as("SELECT token_digest FROM client_access_tokens")
        .fetch_one(&db)
        .await
        .expect("stored digest");
    assert_ne!(stored.0, token);
    assert_eq!(stored.0.len(), 64);

    let current = post_json(
        &app,
        "/api/currentUser",
        json!({"id": "100001", "uuid": "client-a"}),
        Some(token),
    )
    .await;
    assert_eq!(current.status(), StatusCode::OK);
    let current = response_json(current).await;
    assert_eq!(current["name"], "admin");
    assert!(current.get("user").is_none());

    let replacement = post_json(
        &app,
        "/api/login",
        login_body("admin", "test-password", "100001", "client-a"),
        None,
    )
    .await;
    assert_eq!(replacement.status(), StatusCode::OK);
    let replacement = response_json(replacement).await;
    let replacement_token = replacement["access_token"].as_str().expect("token");
    assert_ne!(replacement_token, token);
    let rotated = post_json(
        &app,
        "/api/currentUser",
        json!({"id": "100001", "uuid": "client-a"}),
        Some(token),
    )
    .await;
    assert_eq!(rotated.status(), StatusCode::UNAUTHORIZED);

    let wrong_identity = post_json(
        &app,
        "/api/currentUser",
        json!({"id": "100001", "uuid": "client-b"}),
        Some(replacement_token),
    )
    .await;
    assert_eq!(wrong_identity.status(), StatusCode::UNAUTHORIZED);

    let logout = post_json(
        &app,
        "/api/logout",
        json!({"id": "100001", "uuid": "client-a"}),
        Some(replacement_token),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::OK);
    let revoked = post_json(
        &app,
        "/api/currentUser",
        json!({"id": "100001", "uuid": "client-a"}),
        Some(replacement_token),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_or_malformed_token_expiry_is_rejected() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let login = post_json(
        &app,
        "/api/login",
        login_body("admin", "test-password", "100003", "client-expired"),
        None,
    )
    .await;
    let token = response_json(login).await["access_token"]
        .as_str()
        .expect("token")
        .to_string();
    for expiry in ["2020-01-01T00:00:00Z", "not-a-timestamp"] {
        sqlx::query("UPDATE client_access_tokens SET expires_at = ?")
            .bind(expiry)
            .execute(&db)
            .await
            .expect("expiry");
        let response = post_json(
            &app,
            "/api/currentUser",
            json!({"id": "100003", "uuid": "client-expired"}),
            Some(&token),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn login_rejects_noncanonical_and_invalid_credentials_without_issuing_tokens() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let app = build_router(state);
    let invalid = post_json(
        &app,
        "/api/login",
        login_body("admin", "wrong-password", "100001", "client-a"),
        None,
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response_json(invalid).await["error"],
        "Invalid username or password"
    );

    let mut unknown = login_body("admin", "test-password", "100001", "client-a");
    unknown["unsupported"] = json!(true);
    assert_eq!(
        post_json(&app, "/api/login", unknown, None).await.status(),
        StatusCode::BAD_REQUEST
    );
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM client_access_tokens")
        .fetch_one(&db)
        .await
        .expect("count");
    assert_eq!(count.0, 0);

    let mut desktop = login_body("admin", "test-password", "100001", "desktop-a");
    desktop.as_object_mut().expect("object").remove("autoLogin");
    assert_eq!(
        post_json(&app, "/api/login", desktop, None).await.status(),
        StatusCode::OK
    );

    let challenge = json!({
        "username": "admin",
        "id": "100001",
        "uuid": "desktop-a",
        "type": "email_code",
        "verificationCode": "123456",
        "deviceInfo": {}
    });
    assert_eq!(
        post_json(&app, "/api/login", challenge, None)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn client_login_upgrades_imported_bcrypt_once_and_activates_normal_flow() {
    let state = common::test_state().await;
    let db = state.db.clone();
    let user_uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (user_uuid, username, password_hash, role, activation_state, created_at, updated_at)
         VALUES (?, 'migrated', '', 'operator', 'active', 'test', 'test')",
    )
    .bind(user_uuid.to_string())
    .execute(&db)
    .await
    .expect("user");
    let target_uuid = Uuid::new_v4();
    sqlx::query("INSERT INTO opendesk_instance (instance_uuid) VALUES (?)")
        .bind(target_uuid.to_string())
        .execute(&db)
        .await
        .expect("instance");
    let run_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO migration_runs (
            run_id, source_system, source_instance, source_export_id, source_snapshot_sha256,
            input_sha256, target_instance_uuid, target_state_sha256, backup_sha256, plan_sha256,
            status, started_at, completed_at
         ) VALUES (?, 'test', 'test', 'test', ?, ?, ?, ?, ?, ?, 'applied', 'test', 'test')",
    )
    .bind(run_id.to_string())
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind(target_uuid.to_string())
    .bind("c".repeat(64))
    .bind("d".repeat(64))
    .bind("e".repeat(64))
    .execute(&db)
    .await
    .expect("run");
    sqlx::query(
        "INSERT INTO migration_legacy_credentials
         (user_uuid, run_id, verifier_algorithm, verifier, created_at)
         VALUES (?, ?, 'bcrypt', ?, 'test')",
    )
    .bind(user_uuid.to_string())
    .bind(run_id.to_string())
    .bind(hash("preserved-password", 6).expect("bcrypt"))
    .execute(&db)
    .await
    .expect("credential");
    let app = build_router(state);

    let response = post_json(
        &app,
        "/api/login",
        login_body(
            "migrated",
            "preserved-password",
            "100002",
            "client-migrated",
        ),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let user = find_user_by_username(&db, "migrated")
        .await
        .expect("query")
        .expect("user");
    assert!(user.password_hash.starts_with("$argon2"));
    let pending: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM migration_legacy_credentials WHERE user_uuid = ? AND consumed_at IS NULL",
    )
    .bind(user_uuid.to_string())
    .fetch_one(&db)
    .await
    .expect("pending");
    assert_eq!(pending.0, 0);
}
