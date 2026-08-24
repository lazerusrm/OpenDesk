mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{csrf_token_from_cookie, login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::backup::{parse_backup_json, render_backup_json, BACKUP_SCHEMA_VERSION};
use opendesk::repository::backup::{export_backup_document, restore_backup_document};
use tower::ServiceExt;

#[tokio::test]
async fn backup_export_requires_auth() {
    let state = test_state().await;
    let app = build_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/backup/export.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("backup export");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn backup_export_and_restore_round_trip() {
    let source = test_state().await;
    let site = opendesk::repository::sites::create_site(
        &source.db,
        &opendesk::domain::site::SiteDraft {
            name: "Backup Site".to_string(),
        },
    )
    .await
    .expect("create site");
    let device = opendesk::repository::devices::create_device(
        &source.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Backup Device".to_string(),
            rustdesk_id: Some("424242424".to_string()),
            site_uuid: Some(site.site_uuid),
            notes: Some("restore me".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("create device");

    let admin = opendesk::repository::users::find_user_by_username(&source.db, "admin")
        .await
        .expect("find admin")
        .expect("admin");
    let access_group_uuid = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, ?)")
        .bind(access_group_uuid.to_string())
        .bind("Backup Operators")
        .execute(&source.db)
        .await
        .expect("create access group");
    sqlx::query(
        "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
    )
    .bind(access_group_uuid.to_string())
    .bind(admin.user_uuid.to_string())
    .execute(&source.db)
    .await
    .expect("create membership");
    sqlx::query(
        "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)",
    )
    .bind(access_group_uuid.to_string())
    .bind(device.device_uuid.to_string())
    .execute(&source.db)
    .await
    .expect("create group visibility grant");
    sqlx::query("INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)")
        .bind(admin.user_uuid.to_string())
        .bind(device.device_uuid.to_string())
        .execute(&source.db)
        .await
        .expect("create direct visibility grant");
    let address_book_uuid = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name) VALUES (?, ?, ?)",
    )
    .bind(address_book_uuid.to_string())
    .bind(admin.user_uuid.to_string())
    .bind("Backup Favorites")
    .execute(&source.db)
    .await
    .expect("create address book");
    let entry_uuid = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO address_book_entries
         (address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(entry_uuid.to_string())
    .bind(address_book_uuid.to_string())
    .bind(device.device_uuid.to_string())
    .bind("Backup Device")
    .bind("saved entry")
    .bind(0_i64)
    .execute(&source.db)
    .await
    .expect("create address book entry");
    sqlx::query(
        "INSERT INTO address_book_tags (address_book_uuid, name, color) VALUES (?, 'Critical', 7)",
    )
    .bind(address_book_uuid.to_string())
    .execute(&source.db)
    .await
    .expect("address book tag");
    sqlx::query(
        "INSERT INTO address_book_entry_tags
         (address_book_entry_uuid, address_book_uuid, tag_name) VALUES (?, ?, 'Critical')",
    )
    .bind(entry_uuid.to_string())
    .bind(address_book_uuid.to_string())
    .execute(&source.db)
    .await
    .expect("entry tag");
    sqlx::query("UPDATE users SET activation_state = 'disabled' WHERE user_uuid = ?")
        .bind(admin.user_uuid.to_string())
        .execute(&source.db)
        .await
        .expect("disable user");

    let exported = export_backup_document(&source.db)
        .await
        .expect("export backup");
    assert_eq!(exported.schema_version, BACKUP_SCHEMA_VERSION);
    let json = render_backup_json(&exported).expect("serialize backup");
    let parsed = parse_backup_json(&json).expect("parse backup");

    let target = test_state().await;
    let target_admin = opendesk::repository::users::find_user_by_username(&target.db, "admin")
        .await
        .expect("target admin")
        .expect("target admin");
    opendesk::repository::client_access_tokens::issue_client_access_token(
        &target.db,
        &target.client_token_hmac_key,
        target_admin.user_uuid,
        "111111",
        "restore-client",
        time::OffsetDateTime::now_utc(),
    )
    .await
    .expect("client token");
    restore_backup_document(&target.db, &parsed)
        .await
        .expect("restore backup");
    let restored =
        opendesk::repository::devices::find_device_by_uuid(&target.db, device.device_uuid)
            .await
            .expect("lookup restored device")
            .expect("restored device");
    assert_eq!(restored.alias, "Backup Device");
    assert_eq!(restored.rustdesk_id.as_deref(), Some("424242424"));
    assert_eq!(restored.notes.as_deref(), Some("restore me"));
    assert_eq!(restored.site_uuid, Some(site.site_uuid));
    assert_eq!(exported.access_groups.len(), 1);
    assert_eq!(exported.access_group_memberships.len(), 1);
    assert_eq!(exported.device_visibility_grants.len(), 1);
    assert_eq!(exported.user_device_visibility_grants.len(), 1);
    assert_eq!(exported.address_books.len(), 1);
    assert_eq!(exported.address_book_entries.len(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM address_book_entries")
            .fetch_one(&target.db)
            .await
            .expect("restored address book entry"),
        1,
    );
    let restored_group_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM access_groups")
        .fetch_one(&target.db)
        .await
        .expect("restored access group");
    assert_eq!(restored_group_count, 1);
    let client_token_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM client_access_tokens")
        .fetch_one(&target.db)
        .await
        .expect("client tokens");
    assert_eq!(client_token_count, 0);
    let restored_activation: String =
        sqlx::query_scalar("SELECT activation_state FROM users WHERE user_uuid = ?")
            .bind(admin.user_uuid.to_string())
            .fetch_one(&target.db)
            .await
            .expect("activation state");
    assert_eq!(restored_activation, "disabled");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM address_book_entry_tags")
            .fetch_one(&target.db)
            .await
            .expect("entry tags"),
        1
    );
}

#[tokio::test]
async fn backup_export_json_endpoint_returns_schema_version() {
    let state = test_state().await;
    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let csrf = csrf_token_from_cookie(&session_cookie);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/backup/export.json?csrf_token={csrf}"))
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("backup export");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("\"schema_version\": 3"));
    assert!(json.contains("\"excludes_sessions\": true"));
}
