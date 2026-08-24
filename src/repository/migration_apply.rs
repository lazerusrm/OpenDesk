//! Staging-only atomic import for the canonical sanitized migration contract.

#[path = "migration_apply_storage.rs"]
mod migration_apply_storage;
#[path = "migration_apply_visibility.rs"]
mod migration_apply_visibility;

use migration_apply_storage::{bind, insert_user, reject_collision};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::{uuid, Uuid};

use crate::{
    domain::{
        migration_apply_guard::{MigrationApplyGuardError, StagingApplyGuard},
        migration_apply_plan::MigrationApplyPlan,
        migration_contract::{
            AddressBookKind, AddressBookPermission, AddressBookPrincipalType, MigrationManifest,
            SanitizedMigrationExport,
        },
        migration_preflight::MigrationPreflight,
        role::Role,
    },
    time_format::format_timestamp,
};

use super::migration_credentials::import_protected_credentials;

const MIGRATION_NAMESPACE: Uuid = uuid!("70863e40-869e-4aec-bcdd-b4b0fb0395f8");
pub const ACTIVATION_POLICY_ACTIVATE_ALL: &str = "activate_all";

#[derive(Debug, Error)]
pub enum MigrationApplyError {
    #[error("migration apply database error")]
    Database(#[from] sqlx::Error),
    #[error("migration apply contract failed")]
    Contract(#[from] crate::domain::migration_contract::MigrationContractError),
    #[error("migration apply guard failed")]
    Guard(#[from] MigrationApplyGuardError),
    #[error("migration apply manifest does not match signed plan")]
    ManifestMismatch,
    #[error("migration apply source has unsupported semantics")]
    UnsupportedSemantics,
    #[error("migration apply target collision")]
    Collision,
    #[error("migration apply protected credential artifact failed")]
    Credentials(#[from] crate::domain::migration_credentials::CredentialArtifactError),
    #[error("migration apply protected credential set mismatch")]
    CredentialSetMismatch,
    #[error("migration apply protected credential artifact is not bound to this plan")]
    CredentialBindingMismatch,
    #[error("migration apply protected credential run is not applied")]
    CredentialRunMismatch,
    #[error("migration apply protected credential record has no source binding")]
    CredentialBindingMissing,
    #[error("migration apply protected credential artifact is required by plan")]
    CredentialArtifactRequired,
    #[error("migration apply source role is invalid")]
    InvalidRole,
    #[error("migration apply replay is refused")]
    Replay,
}

pub async fn apply_staging_migration(
    database: &std::path::Path,
    backup: &std::path::Path,
    input: &[u8],
    export: &SanitizedMigrationExport,
    manifest: &MigrationManifest,
    preflight: &MigrationPreflight,
    plan: &MigrationApplyPlan,
    public_key: &[u8],
    signature: &[u8],
    now: OffsetDateTime,
) -> Result<(), MigrationApplyError> {
    apply_staging_migration_with_credentials(
        database, backup, input, export, manifest, preflight, plan, public_key, signature, now,
        None,
    )
    .await
}

pub async fn apply_staging_migration_with_credentials(
    database: &std::path::Path,
    backup: &std::path::Path,
    input: &[u8],
    export: &SanitizedMigrationExport,
    manifest: &MigrationManifest,
    preflight: &MigrationPreflight,
    plan: &MigrationApplyPlan,
    public_key: &[u8],
    signature: &[u8],
    now: OffsetDateTime,
    credentials: Option<&[u8]>,
) -> Result<(), MigrationApplyError> {
    let parsed_input = crate::domain::migration_contract::parse_sanitized_export(
        std::str::from_utf8(input).map_err(|_| {
            MigrationApplyError::Contract(
                crate::domain::migration_contract::MigrationContractError::InvalidSourceValue {
                    field: "json",
                },
            )
        })?,
    )?;
    if &parsed_input != export {
        return Err(MigrationApplyError::CredentialBindingMismatch);
    }
    crate::domain::migration_contract::validate_manifest(manifest, export)?;
    let manifest_bytes =
        serde_json::to_vec(manifest).map_err(|_| MigrationApplyError::UnsupportedSemantics)?;
    if crate::domain::migration_contract::sha256_digest(&manifest_bytes) != plan.manifest_sha256 {
        return Err(MigrationApplyError::ManifestMismatch);
    }
    if !export.settings.is_empty() {
        return Err(MigrationApplyError::UnsupportedSemantics);
    }
    if let Some(expected) = &plan.credential_artifact_sha256 {
        let bytes = match credentials {
            Some(value) => value,
            None if plan.credential_artifact_sha256.is_some() => {
                return Err(MigrationApplyError::CredentialArtifactRequired)
            }
            None => return Ok(()),
        };
        if crate::domain::migration_contract::sha256_digest(bytes) != *expected {
            return Err(MigrationApplyError::CredentialBindingMismatch);
        }
        let artifact = crate::domain::migration_credentials::parse_artifact(bytes)?;
        if artifact.input_sha256 != preflight.input_sha256
            || artifact.source_system != export.provenance.source_system
            || artifact.source_instance != export.provenance.source_instance
            || artifact.source_export_id != export.provenance.source_export_id
            || artifact.run_id != export.run.run_id
            || artifact.target_instance_uuid
                != Uuid::parse_str(&preflight.target_instance_uuid)
                    .map_err(|_| MigrationApplyError::CredentialBindingMismatch)?
            || plan.credential_activation_policy.as_deref()
                != Some(artifact.activation_policy.as_str())
        {
            return Err(MigrationApplyError::CredentialBindingMismatch);
        }
    } else if credentials.is_some() {
        return Err(MigrationApplyError::CredentialBindingMismatch);
    }
    let mut guard = StagingApplyGuard::acquire(
        database, backup, input, preflight, plan, public_key, signature, now,
    )
    .await?;
    let result = apply_locked(
        guard.connection(),
        export,
        preflight,
        plan,
        now,
        credentials,
    )
    .await;
    if let Err(error) = result {
        guard.rollback().await?;
        return Err(error);
    }
    guard.commit().await?;
    Ok(())
}

async fn apply_locked(
    connection: &mut sqlx::SqliteConnection,
    export: &SanitizedMigrationExport,
    preflight: &MigrationPreflight,
    plan: &MigrationApplyPlan,
    now: OffsetDateTime,
    credentials: Option<&[u8]>,
) -> Result<(), MigrationApplyError> {
    let replay: Option<(String,)> = sqlx::query_as(
        "SELECT run_id FROM migration_runs WHERE source_instance = ? AND input_sha256 = ?",
    )
    .bind(&export.provenance.source_instance)
    .bind(&preflight.input_sha256)
    .fetch_optional(&mut *connection)
    .await?;
    if replay.is_some() {
        return Err(MigrationApplyError::Replay);
    }

    let timestamp = format_timestamp(now);
    sqlx::query("INSERT INTO migration_runs (run_id, source_system, source_instance, source_export_id, source_snapshot_sha256, input_sha256, target_instance_uuid, target_state_sha256, backup_sha256, plan_sha256, status, started_at, completed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'applied', ?, ?)")
        .bind(export.run.run_id.to_string()).bind(&export.provenance.source_system).bind(&export.provenance.source_instance)
        .bind(&export.provenance.source_export_id).bind(&export.provenance.snapshot_sha256).bind(&preflight.input_sha256)
        .bind(&preflight.target_instance_uuid).bind(&preflight.target_snapshot_sha256).bind(&preflight.backup_sha256)
        .bind(crate::domain::migration_contract::sha256_digest(&plan.canonical_bytes().map_err(|_| MigrationApplyError::UnsupportedSemantics)?)).bind(&timestamp).bind(&timestamp).execute(&mut *connection).await?;

    for user in &export.users {
        let role = user.role.as_deref().unwrap_or(Role::READ_ONLY);
        Role::parse(role).map_err(|_| MigrationApplyError::InvalidRole)?;
        let user_uuid = source_uuid("user", &user.source_user_id);
        reject_collision(connection, "users", "username", &user.username).await?;
        reject_collision(connection, "users", "user_uuid", &user_uuid.to_string()).await?;
        insert_user(connection, user_uuid, &user.username, role, &timestamp).await?;
        bind(connection, export, "user", &user.source_user_id, user_uuid).await?;
    }
    import_protected_credentials(connection, export, preflight, plan, now, credentials).await?;
    for group in &export.groups {
        let group_uuid = source_uuid("group", &group.source_group_id);
        reject_collision(
            connection,
            "access_groups",
            "access_group_uuid",
            &group_uuid.to_string(),
        )
        .await?;
        sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, ?)")
            .bind(group_uuid.to_string())
            .bind(&group.name)
            .execute(&mut *connection)
            .await?;
        bind(
            connection,
            export,
            "group",
            &group.source_group_id,
            group_uuid,
        )
        .await?;
    }
    for membership in &export.user_group_memberships {
        sqlx::query(
            "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
        )
        .bind(source_uuid("group", &membership.source_group_id).to_string())
        .bind(source_uuid("user", &membership.source_user_id).to_string())
        .execute(&mut *connection)
        .await?;
    }
    for device in &export.devices {
        let device_uuid = source_uuid("device", &device.rustdesk_id);
        reject_collision(connection, "devices", "rustdesk_id", &device.rustdesk_id).await?;
        reject_collision(
            connection,
            "devices",
            "device_uuid",
            &device_uuid.to_string(),
        )
        .await?;
        sqlx::query(
            "INSERT INTO devices
             (device_uuid, rustdesk_id, alias, hostname, owner, archived, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
        )
        .bind(device_uuid.to_string())
        .bind(&device.rustdesk_id)
        .bind(&device.alias)
        .bind(&device.hostname)
        .bind(
            device
                .owner_source_user_id
                .as_deref()
                .and_then(|owner| {
                    export
                        .users
                        .iter()
                        .find(|user| user.source_user_id == owner)
                })
                .map(|user| user.username.as_str()),
        )
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *connection)
        .await?;
        bind(
            connection,
            export,
            "device",
            &device.rustdesk_id,
            device_uuid,
        )
        .await?;
        for group_id in &device.source_group_ids {
            sqlx::query("INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)")
                .bind(source_uuid("group", group_id).to_string()).bind(device_uuid.to_string())
                .execute(&mut *connection).await?;
        }
        migration_apply_visibility::apply_device_visibility(
            connection,
            export,
            device_uuid,
            device.owner_source_user_id.as_deref(),
        )
        .await?;
    }
    for book in &export.address_books {
        let owner = book
            .owner_source_user_id
            .as_ref()
            .ok_or(MigrationApplyError::UnsupportedSemantics)?;
        let book_uuid = source_uuid("address_book", &book.source_address_book_id);
        reject_collision(
            connection,
            "address_books",
            "address_book_uuid",
            &book_uuid.to_string(),
        )
        .await?;
        let name_collision: Option<(String,)> = sqlx::query_as(
            "SELECT address_book_uuid FROM address_books WHERE owner_user_uuid = ? AND name = ?",
        )
        .bind(source_uuid("user", owner).to_string())
        .bind(&book.name)
        .fetch_optional(&mut *connection)
        .await?;
        if name_collision.is_some() {
            return Err(MigrationApplyError::Collision);
        }
        sqlx::query(
            "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name, book_kind)
             VALUES (?, ?, ?, ?)",
        )
        .bind(book_uuid.to_string())
        .bind(source_uuid("user", owner).to_string())
        .bind(&book.name)
        .bind(match book.book_kind {
            AddressBookKind::Personal => "personal",
            AddressBookKind::Shared => "shared",
        })
        .execute(&mut *connection)
        .await?;
        for rule in &book.rules {
            let (principal_type, principal_uuid) = match rule.principal_type {
                AddressBookPrincipalType::User => {
                    ("user", source_uuid("user", &rule.principal_id).to_string())
                }
                AddressBookPrincipalType::Group => (
                    "group",
                    source_uuid("group", &rule.principal_id).to_string(),
                ),
            };
            let permission = match rule.permission {
                AddressBookPermission::Read => "read",
                AddressBookPermission::Write => "write",
                AddressBookPermission::Admin => "admin",
            };
            sqlx::query(
                "INSERT INTO address_book_access_rules
                 (address_book_uuid, principal_type, principal_uuid, permission)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(book_uuid.to_string())
            .bind(principal_type)
            .bind(principal_uuid)
            .bind(permission)
            .execute(&mut *connection)
            .await?;
        }
        bind(
            connection,
            export,
            "address_book",
            &book.source_address_book_id,
            book_uuid,
        )
        .await?;
    }
    for (position, entry) in export.address_book_entries.iter().enumerate() {
        let source_id = format!("{}:{}", entry.source_address_book_id, entry.rustdesk_id);
        let entry_uuid = source_uuid("address_book_entry", &source_id);
        sqlx::query("INSERT INTO address_book_entries (address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(entry_uuid.to_string()).bind(source_uuid("address_book", &entry.source_address_book_id).to_string())
            .bind(source_uuid("device", &entry.rustdesk_id).to_string()).bind(&entry.alias).bind(&entry.notes).bind(position as i64)
            .execute(&mut *connection).await?;
        bind(
            connection,
            export,
            "address_book_entry",
            &source_id,
            entry_uuid,
        )
        .await?;
    }
    Ok(())
}

fn source_uuid(kind: &str, source_id: &str) -> Uuid {
    Uuid::new_v5(
        &MIGRATION_NAMESPACE,
        format!("{kind}:{source_id}").as_bytes(),
    )
}
