use std::collections::BTreeMap;
use std::fs;

use bcrypt::hash;
use ed25519_dalek::{Signer, SigningKey};
use opendesk::{
    auth,
    domain::{
        migration_apply_plan::{MigrationApplyPlan, MIGRATION_APPLY_PLAN_SCHEMA_VERSION},
        migration_contract::{
            AddressBookKind, MigrationManifest, MigrationRunState, MigrationRunStatus,
            SanitizedMigrationExport, SourceAddressBook, SourceAddressBookEntry, SourceDevice,
            SourceGroup, SourceProvenance, SourceUser, SourceUserGroupMembership,
            SANITIZED_MIGRATION_SCHEMA_VERSION,
        },
        migration_credentials::{
            ProtectedCredentialArtifact, ProtectedCredentialRecord, ACTIVATION_POLICY_ACTIVATE_ALL,
            CREDENTIAL_ARTIFACT_SCHEMA_VERSION,
        },
        migration_preflight::capture_preflight,
    },
    repository::{
        migration_apply::{apply_staging_migration_with_credentials, MigrationApplyError},
        users::{find_legacy_credential, find_user_by_username, upgrade_legacy_credential},
    },
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
            source_group_ids: vec!["group-1".into()],
        }],
        address_books: vec![SourceAddressBook {
            source_address_book_id: "book-1".into(),
            name: "Book".into(),
            owner_source_user_id: Some("user-1".into()),
            book_kind: AddressBookKind::Personal,
            rules: vec![],
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

fn credential_artifact(
    export: &SanitizedMigrationExport,
    preflight: &opendesk::domain::migration_preflight::MigrationPreflight,
    input: &[u8],
    activation_policy: &str,
) -> ProtectedCredentialArtifact {
    ProtectedCredentialArtifact {
        schema_version: CREDENTIAL_ARTIFACT_SCHEMA_VERSION,
        source_system: export.provenance.source_system.clone(),
        source_instance: export.provenance.source_instance.clone(),
        source_export_id: export.provenance.source_export_id.clone(),
        run_id: export.run.run_id,
        target_instance_uuid: Uuid::parse_str(&preflight.target_instance_uuid)
            .expect("target uuid"),
        input_sha256: opendesk::domain::migration_contract::sha256_digest(input),
        activation_policy: activation_policy.to_string(),
        records: vec![ProtectedCredentialRecord {
            source_user_id: "user-1".into(),
            verifier_algorithm: "bcrypt".into(),
            verifier: hash("synthetic-migration-password", 6).expect("synthetic bcrypt"),
        }],
    }
}

fn exporter_style_bytes(artifact: &ProtectedCredentialArtifact) -> Vec<u8> {
    let value = serde_json::to_value(artifact).expect("artifact value");
    let object = value.as_object().expect("artifact object");
    let sorted: BTreeMap<_, _> = object.iter().map(|(key, value)| (key, value)).collect();
    let mut bytes = serde_json::to_vec(&sorted).expect("sorted artifact");
    bytes.push(b'\n');
    bytes
}

#[tokio::test]
async fn signed_activate_all_apply_preserves_one_time_upgrade_and_restore_lifecycle() {
    let (database, backup, export, input, preflight, mut plan, signing_key, _) = fixture().await;
    let artifact = credential_artifact(&export, &preflight, &input, ACTIVATION_POLICY_ACTIVATE_ALL);
    let artifact_bytes = exporter_style_bytes(&artifact);
    plan.credential_artifact_sha256 = Some(opendesk::domain::migration_contract::sha256_digest(
        &artifact_bytes,
    ));
    plan.credential_activation_policy = Some(ACTIVATION_POLICY_ACTIVATE_ALL.into());
    let signature = signing_key
        .sign(&plan.canonical_bytes().expect("signed plan"))
        .to_bytes();
    apply_staging_migration_with_credentials(
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
        Some(&artifact_bytes),
    )
    .await
    .expect("signed credential apply");

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}", database.display()))
        .await
        .expect("pool");
    let user = find_user_by_username(&pool, "migrated")
        .await
        .expect("user query")
        .expect("migrated user");
    assert_eq!(user.activation_state, "active");
    let legacy = find_legacy_credential(&pool, user.user_uuid)
        .await
        .expect("legacy query")
        .expect("pending verifier");
    assert_eq!(legacy.algorithm, "bcrypt");
    assert!(auth::verify_legacy_bcrypt(
        "synthetic-migration-password",
        &legacy.algorithm,
        &legacy.verifier
    )
    .is_ok());

    let backup_document = opendesk::repository::backup::export_backup_document(&pool)
        .await
        .expect("backup export");
    assert!(matches!(
        opendesk::repository::backup::restore_backup_document(&pool, &backup_document).await,
        Err(opendesk::repository::backup::BackupRestoreError::UnconsumedMigrationVerifiers)
    ));
    assert!(upgrade_legacy_credential(
        &pool,
        user.user_uuid,
        &legacy.algorithm,
        &legacy.verifier,
        "synthetic-migration-password",
    )
    .await
    .expect("first login upgrade"));
    assert!(!upgrade_legacy_credential(
        &pool,
        user.user_uuid,
        &legacy.algorithm,
        &legacy.verifier,
        "synthetic-migration-password",
    )
    .await
    .expect("replay upgrade query"));
    assert!(find_legacy_credential(&pool, user.user_uuid)
        .await
        .expect("consumed query")
        .is_none());
    opendesk::repository::backup::restore_backup_document(&pool, &backup_document)
        .await
        .expect("restore after verifier consumption");
    pool.close().await;
    let _ = fs::remove_file(database);
    let _ = fs::remove_file(backup);
}

#[tokio::test]
async fn protected_credential_plan_requires_exact_artifact_and_rolls_back() {
    let (database, backup, export, input, preflight, mut plan, signing_key, _) = fixture().await;
    let artifact = credential_artifact(&export, &preflight, &input, ACTIVATION_POLICY_ACTIVATE_ALL);
    let artifact_bytes = exporter_style_bytes(&artifact);
    plan.credential_artifact_sha256 = Some(opendesk::domain::migration_contract::sha256_digest(
        &artifact_bytes,
    ));
    plan.credential_activation_policy = Some(ACTIVATION_POLICY_ACTIVATE_ALL.into());
    let signature = signing_key
        .sign(&plan.canonical_bytes().expect("signed plan"))
        .to_bytes();
    let tampered = serde_json::to_string_pretty(&artifact)
        .expect("semantic artifact")
        .into_bytes();
    let error = apply_staging_migration_with_credentials(
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
        Some(&tampered),
    )
    .await
    .expect_err("tampered artifact");
    assert!(matches!(
        error,
        MigrationApplyError::CredentialBindingMismatch
    ));
    let mut connection = sqlx::SqliteConnection::connect_with(
        &sqlx::sqlite::SqliteConnectOptions::new().filename(&database),
    )
    .await
    .expect("read rollback");
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM users), (SELECT COUNT(*) FROM migration_runs)",
    )
    .fetch_one(&mut connection)
    .await
    .expect("counts");
    assert_eq!(counts, (0, 0));
    connection.close().await.expect("close");
    let _ = fs::remove_file(database);
    let _ = fs::remove_file(backup);
}
