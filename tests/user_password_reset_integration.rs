use opendesk::{
    auth::verify_password,
    repository::users::{reset_user_password, ResetUserPasswordError},
};
use sqlx::SqlitePool;
use time::{macros::datetime, OffsetDateTime};
use uuid::Uuid;

async fn pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.expect("pool");
    sqlx::migrate!().run(&pool).await.expect("migrations");
    pool
}

async fn insert_user(pool: &SqlitePool, username: &str, state: &str) -> String {
    let user_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (user_uuid, username, password_hash, role, activation_state, created_at, updated_at) \
         VALUES (?, ?, 'old-hash', 'operator', ?, 'old', 'old')",
    )
    .bind(&user_uuid)
    .bind(username)
    .bind(state)
    .execute(pool)
    .await
    .expect("user");
    user_uuid
}

async fn insert_runtime_state(pool: &SqlitePool, user_uuid: &str) {
    sqlx::query(
        "INSERT INTO sessions (session_uuid, user_uuid, expires_at, created_at, csrf_token) \
         VALUES (?, ?, 'future', 'now', 'csrf')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_uuid)
    .execute(pool)
    .await
    .expect("session");
    sqlx::query(
        "INSERT INTO client_access_tokens (client_access_token_uuid, token_digest, user_uuid, \
         rustdesk_id, client_uuid, expires_at, created_at, last_used_at) \
         VALUES (?, ?, ?, '123', 'client', 'future', 'now', 'now')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind("a".repeat(64))
    .bind(user_uuid)
    .execute(pool)
    .await
    .expect("token");
}

#[tokio::test]
async fn reset_consumes_legacy_state_revokes_access_and_audits_without_detail() {
    let pool = pool().await;
    let user_uuid = insert_user(&pool, "ExactUser", "active").await;
    insert_runtime_state(&pool, &user_uuid).await;
    let run_uuid = Uuid::new_v4().to_string();
    let instance_uuid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO opendesk_instance (instance_uuid) VALUES (?)")
        .bind(&instance_uuid)
        .execute(&pool)
        .await
        .expect("instance");
    sqlx::query(
        "INSERT INTO migration_runs (run_id, source_system, source_instance, source_export_id, \
         source_snapshot_sha256, input_sha256, target_instance_uuid, target_state_sha256, \
         backup_sha256, plan_sha256, status, started_at, completed_at) \
         VALUES (?, 'source', 'instance', 'export', 'a', 'b', ?, 'c', 'd', 'e', 'applied', 'old', 'old')",
    )
    .bind(&run_uuid)
    .bind(&instance_uuid)
    .execute(&pool)
    .await
    .expect("run");
    sqlx::query(
        "INSERT INTO migration_legacy_credentials \
         (user_uuid, run_id, verifier_algorithm, verifier, created_at) \
         VALUES (?, ?, 'bcrypt', ?, 'old')",
    )
    .bind(&user_uuid)
    .bind(&run_uuid)
    .bind("$2b$06$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    .execute(&pool)
    .await
    .expect("legacy credential");

    let now = datetime!(2026-07-23 12:34:56 UTC);
    reset_user_password(&pool, "ExactUser", "new-password", now)
        .await
        .expect("reset");

    let (hash, updated_at): (String, String) =
        sqlx::query_as("SELECT password_hash, updated_at FROM users WHERE user_uuid = ?")
            .bind(&user_uuid)
            .fetch_one(&pool)
            .await
            .expect("user state");
    assert!(verify_password("new-password", &hash).is_ok());
    assert_eq!(updated_at, "2026-07-23T12:34:56Z");
    let consumed_at: Option<String> = sqlx::query_scalar(
        "SELECT consumed_at FROM migration_legacy_credentials WHERE user_uuid = ?",
    )
    .bind(&user_uuid)
    .fetch_one(&pool)
    .await
    .expect("credential");
    assert_eq!(consumed_at.as_deref(), Some("2026-07-23T12:34:56Z"));
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_uuid = ?")
        .bind(&user_uuid)
        .fetch_one(&pool)
        .await
        .expect("sessions");
    assert_eq!(sessions, 0);
    let revoked_at: Option<String> =
        sqlx::query_scalar("SELECT revoked_at FROM client_access_tokens WHERE user_uuid = ?")
            .bind(&user_uuid)
            .fetch_one(&pool)
            .await
            .expect("token");
    assert_eq!(revoked_at.as_deref(), Some("2026-07-23T12:34:56Z"));
    let audit: (String, String, Option<String>, String) =
        sqlx::query_as("SELECT action, source, detail_json, object_uuid FROM audit_events")
            .fetch_one(&pool)
            .await
            .expect("audit");
    assert_eq!(
        audit,
        (
            "password_reset".into(),
            "operator_cli".into(),
            None,
            user_uuid
        )
    );
}

#[tokio::test]
async fn missing_or_inactive_user_changes_nothing() {
    let pool = pool().await;
    let user_uuid = insert_user(&pool, "Inactive", "disabled").await;
    insert_runtime_state(&pool, &user_uuid).await;

    for username in ["Missing", "Inactive"] {
        assert!(matches!(
            reset_user_password(&pool, username, "new-password", OffsetDateTime::now_utc()).await,
            Err(ResetUserPasswordError::MissingOrInactive)
        ));
    }

    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE user_uuid = ?")
        .bind(&user_uuid)
        .fetch_one(&pool)
        .await
        .expect("hash");
    assert_eq!(hash, "old-hash");
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&pool)
        .await
        .expect("sessions");
    let revoked: Option<String> = sqlx::query_scalar("SELECT revoked_at FROM client_access_tokens")
        .fetch_one(&pool)
        .await
        .expect("token");
    let audits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
        .fetch_one(&pool)
        .await
        .expect("audits");
    assert_eq!((sessions, revoked, audits), (1, None, 0));
}
