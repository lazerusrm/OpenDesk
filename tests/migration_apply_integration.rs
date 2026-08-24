use std::fs;

use ed25519_dalek::{Signer, SigningKey};
use opendesk::{
    domain::{
        migration_apply_plan::{MigrationApplyPlan, MIGRATION_APPLY_PLAN_SCHEMA_VERSION},
        migration_contract::{
            AddressBookKind, AddressBookPermission, AddressBookPrincipalType, MigrationManifest,
            MigrationRunState, MigrationRunStatus, SanitizedMigrationExport, SourceAddressBook,
            SourceAddressBookEntry, SourceAddressBookRule, SourceDevice, SourceGroup,
            SourceProvenance, SourceUser, SourceUserGroupMembership,
            SANITIZED_MIGRATION_SCHEMA_VERSION,
        },
        migration_preflight::capture_preflight,
    },
    repository::migration_apply::{apply_staging_migration, MigrationApplyError},
};
use sqlx::{sqlite::SqliteConnectOptions, Connection};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

async fn fixture() -> (
    std::path::PathBuf,
    std::path::PathBuf,
    SanitizedMigrationExport,
    Vec<u8>,
    opendesk::domain::migration_preflight::MigrationPreflight,
    MigrationApplyPlan,
    SigningKey,
    [u8; 64],
) {
    let database = std::env::temp_dir().join(format!("opendesk-apply-{}.sqlite", Uuid::new_v4()));
    let backup = std::path::PathBuf::from(format!("{}.backup", database.display()));
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
    let instance_uuid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO opendesk_instance (instance_uuid) VALUES (?)")
        .bind(&instance_uuid)
        .execute(&mut connection)
        .await
        .expect("instance");
    sqlx::query(
        "INSERT INTO migration_staging_targets (instance_uuid, marked_at) VALUES (?, 'test')",
    )
    .bind(&instance_uuid)
    .execute(&mut connection)
    .await
    .expect("staging marker");
    connection.close().await.expect("close");
    fs::copy(&database, &backup).expect("backup");
    fs::set_permissions(
        &database,
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )
    .expect("database mode");
    fs::set_permissions(&backup, std::os::unix::fs::PermissionsExt::from_mode(0o600))
        .expect("backup mode");
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("timestamp");
    let digest = "a".repeat(64);
    let export = SanitizedMigrationExport {
        schema_version: SANITIZED_MIGRATION_SCHEMA_VERSION,
        provenance: SourceProvenance {
            source_system: "test_source".into(),
            source_instance: "test_instance".into(),
            source_export_id: "test_export".into(),
            source_schema_version: "1".into(),
            exported_at: now.clone(),
            snapshot_sha256: digest.clone(),
        },
        run: MigrationRunState {
            run_id: Uuid::new_v4(),
            status: MigrationRunStatus::Approved,
            started_at: now.clone(),
            completed_at: Some(now.clone()),
            source_snapshot_sha256: digest,
        },
        users: vec![SourceUser {
            source_user_id: "user-1".into(),
            username: "migrated".into(),
            role: Some("operator".into()),
            credential_reset_required: true,
        }],
        groups: vec![SourceGroup {
            source_group_id: "group-1".into(),
            name: "Migrated".into(),
            allow_device_access_within_group: true,
        }],
        user_group_memberships: vec![SourceUserGroupMembership {
            source_user_id: "user-1".into(),
            source_group_id: "group-1".into(),
        }],
        cross_group_access: vec![],
        devices: vec![SourceDevice {
            rustdesk_id: "100".into(),
            alias: "Device".into(),
            hostname: Some("host".into()),
            owner_source_user_id: Some("user-1".into()),
            source_group_ids: vec![],
        }],
        address_books: vec![SourceAddressBook {
            source_address_book_id: "book-1".into(),
            name: "Book".into(),
            owner_source_user_id: Some("user-1".into()),
            book_kind: AddressBookKind::Shared,
            rules: vec![SourceAddressBookRule {
                principal_type: AddressBookPrincipalType::Group,
                principal_id: "group-1".into(),
                permission: AddressBookPermission::Write,
            }],
        }],
        address_book_entries: vec![SourceAddressBookEntry {
            source_address_book_id: "book-1".into(),
            rustdesk_id: "100".into(),
            alias: "Device".into(),
            notes: None,
            credential_reset_required: true,
        }],
        settings: vec![],
        unsupported_semantics: vec![],
    };
    let input = serde_json::to_vec(&export).expect("export");
    let preflight = capture_preflight(&database, &input, &export, &backup)
        .await
        .expect("preflight");
    let approved_at = OffsetDateTime::now_utc();
    let plan = MigrationApplyPlan {
        schema_version: MIGRATION_APPLY_PLAN_SCHEMA_VERSION,
        input_sha256: preflight.input_sha256.clone(),
        source_system: preflight.source_system.clone(),
        source_instance: preflight.source_instance.clone(),
        source_export_id: preflight.source_export_id.clone(),
        source_snapshot_sha256: preflight.source_snapshot_sha256.clone(),
        target_instance_uuid: preflight.target_instance_uuid.clone(),
        target_snapshot_sha256: preflight.target_snapshot_sha256.clone(),
        backup_instance_uuid: preflight.backup_instance_uuid.clone(),
        backup_sha256: preflight.backup_sha256.clone(),
        manifest_sha256: opendesk::domain::migration_contract::sha256_digest(
            &serde_json::to_vec(&manifest(&export)).expect("manifest"),
        ),
        credential_artifact_sha256: None,
        credential_activation_policy: None,
        approved_at: approved_at.format(&Rfc3339).expect("approved"),
        expires_at: (approved_at + time::Duration::hours(1))
            .format(&Rfc3339)
            .expect("expiry"),
    };
    let signing_key = SigningKey::from_bytes(&[3; 32]);
    let signature = signing_key
        .sign(&plan.canonical_bytes().expect("plan"))
        .to_bytes();
    (
        database,
        backup,
        export,
        input,
        preflight,
        plan,
        signing_key,
        signature,
    )
}

fn manifest(export: &SanitizedMigrationExport) -> MigrationManifest {
    MigrationManifest {
        schema_version: SANITIZED_MIGRATION_SCHEMA_VERSION,
        run_id: export.run.run_id,
        source_snapshot_sha256: export.provenance.snapshot_sha256.clone(),
        credential_reset_required: true,
        settings_disposition: opendesk::domain::migration_contract::SettingsDisposition::Exclude,
        approved_by: "staging-test".into(),
        approved_at: export.run.started_at.clone(),
    }
}

#[tokio::test]
async fn applies_all_direct_mappings_with_disabled_account_and_provenance() {
    let (database, backup, export, input, preflight, plan, signing_key, signature) =
        fixture().await;
    apply_staging_migration(
        &database,
        &backup,
        &input,
        &export,
        &manifest(&export),
        &preflight,
        &plan,
        signing_key.verifying_key().as_bytes(),
        &signature,
        OffsetDateTime::now_utc(),
    )
    .await
    .expect("apply");
    let mut connection =
        sqlx::SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&database))
            .await
            .expect("read");
    let user: (String, String) =
        sqlx::query_as("SELECT role, activation_state FROM users WHERE username = 'migrated'")
            .fetch_one(&mut connection)
            .await
            .expect("user");
    assert_eq!(user, ("operator".into(), "disabled".into()));
    for table in [
        "migration_runs",
        "migration_source_bindings",
        "access_group_memberships",
        "device_visibility_grants",
        "user_device_visibility_grants",
        "address_books",
        "address_book_access_rules",
        "address_book_entries",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&mut connection)
            .await
            .expect("count");
        assert!(count > 0, "{table}");
    }
    connection.close().await.expect("close");
    let _ = fs::remove_file(database);
    let _ = fs::remove_file(backup);
}

#[tokio::test]
async fn verified_backup_recovers_pre_apply_staging_state() {
    let (database, backup, export, input, preflight, plan, signing_key, signature) =
        fixture().await;
    apply_staging_migration(
        &database,
        &backup,
        &input,
        &export,
        &manifest(&export),
        &preflight,
        &plan,
        signing_key.verifying_key().as_bytes(),
        &signature,
        OffsetDateTime::now_utc(),
    )
    .await
    .expect("apply");
    fs::copy(&backup, &database).expect("restore backup");
    let mut connection =
        sqlx::SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&database))
            .await
            .expect("read");
    let state: (i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM users),
            (SELECT COUNT(*) FROM migration_runs),
            (SELECT COUNT(*) FROM migration_staging_targets)",
    )
    .fetch_one(&mut connection)
    .await
    .expect("state");
    assert_eq!(state, (0, 0, 1));
    connection.close().await.expect("close");
    let _ = fs::remove_file(database);
    let _ = fs::remove_file(backup);
}
#[tokio::test]
async fn late_collision_rolls_back_every_insert() {
    let (database, backup, mut export, _, _, _, signing_key, _) = fixture().await;
    export
        .address_book_entries
        .push(export.address_book_entries[0].clone());
    let input = serde_json::to_vec(&export).expect("export");
    let preflight = capture_preflight(&database, &input, &export, &backup)
        .await
        .expect("preflight");
    let approved_at = OffsetDateTime::now_utc();
    let plan = MigrationApplyPlan {
        schema_version: MIGRATION_APPLY_PLAN_SCHEMA_VERSION,
        input_sha256: preflight.input_sha256.clone(),
        source_system: preflight.source_system.clone(),
        source_instance: preflight.source_instance.clone(),
        source_export_id: preflight.source_export_id.clone(),
        source_snapshot_sha256: preflight.source_snapshot_sha256.clone(),
        target_instance_uuid: preflight.target_instance_uuid.clone(),
        target_snapshot_sha256: preflight.target_snapshot_sha256.clone(),
        backup_instance_uuid: preflight.backup_instance_uuid.clone(),
        backup_sha256: preflight.backup_sha256.clone(),
        manifest_sha256: opendesk::domain::migration_contract::sha256_digest(
            &serde_json::to_vec(&manifest(&export)).expect("manifest"),
        ),
        credential_artifact_sha256: None,
        credential_activation_policy: None,
        approved_at: approved_at.format(&Rfc3339).expect("approved"),
        expires_at: (approved_at + time::Duration::hours(1))
            .format(&Rfc3339)
            .expect("expiry"),
    };
    let signature = signing_key
        .sign(&plan.canonical_bytes().expect("plan"))
        .to_bytes();
    let error = apply_staging_migration(
        &database,
        &backup,
        &input,
        &export,
        &manifest(&export),
        &preflight,
        &plan,
        signing_key.verifying_key().as_bytes(),
        &signature,
        OffsetDateTime::now_utc(),
    )
    .await
    .expect_err("late unique collision");
    assert!(matches!(error, MigrationApplyError::Contract(_)));
    let mut connection =
        sqlx::SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&database))
            .await
            .expect("read");
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&mut connection)
        .await
        .expect("users");
    let runs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM migration_runs")
        .fetch_one(&mut connection)
        .await
        .expect("runs");
    assert_eq!((users, runs), (0, 0));
    connection.close().await.expect("close");
    let _ = fs::remove_file(database);
    let _ = fs::remove_file(backup);
}
