use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use bcrypt::hash;
use opendesk::{
    domain::migration_credentials::{
        ProtectedCredentialArtifact, ProtectedCredentialRecord, ACTIVATION_POLICY_ACTIVATE_ALL,
    },
    repository::{
        migration_credentials::{attach_protected_credentials, CredentialAttachmentError},
        users::{find_legacy_credential, find_user_by_username, upgrade_legacy_credential},
    },
};
use sqlx::{sqlite::SqliteConnectOptions, Connection};
use time::OffsetDateTime;
use uuid::Uuid;

struct Fixture {
    database: PathBuf,
    run_id: Uuid,
    target_instance: Uuid,
    input_sha256: String,
}

impl Fixture {
    async fn create() -> Self {
        let database =
            std::env::temp_dir().join(format!("credential-attach-{}.sqlite", Uuid::new_v4()));
        let mut connection = sqlx::SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true),
        )
        .await
        .expect("connect");
        sqlx::migrate!("./migrations")
            .run(&mut connection)
            .await
            .expect("migrate");
        let run_id = Uuid::new_v4();
        let target_instance = Uuid::new_v4();
        let input_sha256 = "a".repeat(64);
        sqlx::query("INSERT INTO opendesk_instance (instance_uuid) VALUES (?)")
            .bind(target_instance.to_string())
            .execute(&mut connection)
            .await
            .expect("instance");
        sqlx::query("INSERT INTO migration_runs (run_id, source_system, source_instance, source_export_id, source_snapshot_sha256, input_sha256, target_instance_uuid, target_state_sha256, backup_sha256, plan_sha256, status, started_at, completed_at) VALUES (?, 'test_source', 'test_instance', 'test_export', ?, ?, ?, ?, ?, ?, 'applied', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .bind(run_id.to_string()).bind("b".repeat(64)).bind(&input_sha256)
            .bind(target_instance.to_string()).bind("c".repeat(64)).bind("d".repeat(64)).bind("e".repeat(64))
            .execute(&mut connection).await.expect("run");
        sqlx::query("INSERT INTO users (user_uuid, username, password_hash, role, activation_state, created_at, updated_at) VALUES (?, 'migrated', 'disabled-hash', 'operator', 'disabled', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
            .bind(user_uuid()).execute(&mut connection).await.expect("user");
        sqlx::query("INSERT INTO migration_source_bindings (source_system, source_instance, entity_kind, source_id, target_uuid, run_id) VALUES ('test_source', 'test_instance', 'user', 'user-1', ?, ?)")
            .bind(user_uuid()).bind(run_id.to_string()).execute(&mut connection).await.expect("binding");
        connection.close().await.expect("close");
        fs::set_permissions(&database, PermissionsExt::from_mode(0o600)).expect("mode");
        Self {
            database,
            run_id,
            target_instance,
            input_sha256,
        }
    }

    fn artifact(&self) -> Vec<u8> {
        let artifact = ProtectedCredentialArtifact {
            schema_version: 1,
            source_system: "test_source".into(),
            source_instance: "test_instance".into(),
            source_export_id: "test_export".into(),
            run_id: self.run_id,
            target_instance_uuid: self.target_instance,
            input_sha256: self.input_sha256.clone(),
            activation_policy: ACTIVATION_POLICY_ACTIVATE_ALL.into(),
            records: vec![ProtectedCredentialRecord {
                source_user_id: "user-1".into(),
                verifier_algorithm: "bcrypt".into(),
                verifier: hash("synthetic-migration-password", 6).expect("hash"),
            }],
        };
        serde_json::to_vec(&artifact).expect("artifact")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.database);
    }
}

fn user_uuid() -> String {
    "10000000-0000-4000-8000-000000000001".into()
}
fn digest(bytes: &[u8]) -> String {
    opendesk::domain::migration_contract::sha256_digest(bytes)
}

#[tokio::test]
async fn attachment_records_receipt_and_supports_one_login_upgrade() {
    let fixture = Fixture::create().await;
    let bytes = fixture.artifact();
    let artifact_digest = digest(&bytes);
    attach_protected_credentials(
        &fixture.database,
        &bytes,
        &artifact_digest,
        OffsetDateTime::now_utc(),
    )
    .await
    .expect("attach");
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}", fixture.database.display()))
        .await
        .expect("pool");
    let receipt: (String, i64) = sqlx::query_as("SELECT artifact_sha256, record_count FROM migration_credential_attachment_receipts WHERE run_id = ?")
        .bind(fixture.run_id.to_string()).fetch_one(&pool).await.expect("receipt");
    assert_eq!(receipt, (artifact_digest, 1));
    let user = find_user_by_username(&pool, "migrated")
        .await
        .expect("query")
        .expect("user");
    assert_eq!(user.activation_state, "active");
    let credential = find_legacy_credential(&pool, user.user_uuid)
        .await
        .expect("query")
        .expect("credential");
    assert!(upgrade_legacy_credential(
        &pool,
        user.user_uuid,
        &credential.algorithm,
        &credential.verifier,
        "synthetic-migration-password"
    )
    .await
    .expect("upgrade"));
    assert!(find_legacy_credential(&pool, user.user_uuid)
        .await
        .expect("query")
        .is_none());
}

#[tokio::test]
async fn digest_binding_active_user_and_replay_refusals_rollback() {
    let fixture = Fixture::create().await;
    let bytes = fixture.artifact();
    assert_eq!(
        attach_protected_credentials(
            &fixture.database,
            &bytes,
            &"0".repeat(64),
            OffsetDateTime::now_utc()
        )
        .await,
        Err(CredentialAttachmentError::InvalidDigest)
    );
    let mut mismatched: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    mismatched["source_export_id"] = "different-export".into();
    let mismatch_bytes = serde_json::to_vec(&mismatched).expect("json");
    assert_eq!(
        attach_protected_credentials(
            &fixture.database,
            &mismatch_bytes,
            &digest(&mismatch_bytes),
            OffsetDateTime::now_utc()
        )
        .await,
        Err(CredentialAttachmentError::BindingMismatch)
    );
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}", fixture.database.display()))
        .await
        .expect("pool");
    sqlx::query("UPDATE users SET activation_state = 'active' WHERE user_uuid = ?")
        .bind(user_uuid())
        .execute(&pool)
        .await
        .expect("activate");
    assert_eq!(
        attach_protected_credentials(
            &fixture.database,
            &bytes,
            &digest(&bytes),
            OffsetDateTime::now_utc()
        )
        .await,
        Err(CredentialAttachmentError::ActiveUser)
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM migration_legacy_credentials), (SELECT COUNT(*) FROM migration_credential_attachment_receipts)")
        .fetch_one(&pool).await.expect("counts");
    assert_eq!(counts, (0, 0));
    sqlx::query("UPDATE users SET activation_state = 'disabled' WHERE user_uuid = ?")
        .bind(user_uuid())
        .execute(&pool)
        .await
        .expect("disable");
    pool.close().await;
    attach_protected_credentials(
        &fixture.database,
        &bytes,
        &digest(&bytes),
        OffsetDateTime::now_utc(),
    )
    .await
    .expect("attach");
    assert_eq!(
        attach_protected_credentials(
            &fixture.database,
            &bytes,
            &digest(&bytes),
            OffsetDateTime::now_utc()
        )
        .await,
        Err(CredentialAttachmentError::AlreadyAttached)
    );
}
