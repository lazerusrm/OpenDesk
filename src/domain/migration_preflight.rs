//! Immutable preflight binding for a staged migration apply.
//!
//! This module has no write path. It binds the exact source bytes, a verified
//! backup digest, and an exact WAL-free target SQLite snapshot to one initialized
//! OpenDesk instance. A later apply must recheck these values while holding its
//! write lock.

use std::{
    fs,
    io::Read,
    os::unix::{fs::MetadataExt, fs::OpenOptionsExt},
    path::Path,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Connection, Row, SqliteConnection};
use thiserror::Error;

use super::migration_contract::{sha256_digest, SanitizedMigrationExport};

pub const MIGRATION_PREFLIGHT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationPreflight {
    pub schema_version: u32,
    pub input_sha256: String,
    pub source_system: String,
    pub source_instance: String,
    pub source_export_id: String,
    pub source_snapshot_sha256: String,
    pub target_instance_uuid: String,
    pub target_snapshot_sha256: String,
    pub backup_instance_uuid: String,
    pub backup_sha256: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MigrationPreflightError {
    #[error("migration preflight database error")]
    Database,
    #[error("migration preflight target is not an existing regular SQLite file")]
    InvalidTarget,
    #[error("migration preflight target has an active WAL sidecar")]
    ActiveWal,
    #[error("migration preflight backup belongs to a different target instance")]
    BackupTargetMismatch,
    #[error("migration preflight target is not initialized")]
    TargetUninitialized,
    #[error("migration preflight target identity is ambiguous")]
    TargetIdentityAmbiguous,
    #[error("migration preflight value is invalid")]
    InvalidValue,
}

pub async fn capture_preflight(
    database: &Path,
    input: &[u8],
    export: &SanitizedMigrationExport,
    backup: &Path,
) -> Result<MigrationPreflight, MigrationPreflightError> {
    let backup_sha256 = snapshot_digest(backup)?;
    let backup_instance_uuid = instance_uuid_from_database(backup).await?;
    let mut connection = open_immutable_preflight_connection(database).await?;
    let target_snapshot_sha256 = logical_target_snapshot_digest(&mut connection).await?;
    let target_instance_uuid = target_instance_uuid(&mut connection).await?;
    if backup_instance_uuid != target_instance_uuid {
        return Err(MigrationPreflightError::BackupTargetMismatch);
    }
    connection
        .close()
        .await
        .map_err(|_| MigrationPreflightError::Database)?;
    Ok(MigrationPreflight {
        schema_version: MIGRATION_PREFLIGHT_SCHEMA_VERSION,
        input_sha256: sha256_digest(input),
        source_system: export.provenance.source_system.clone(),
        source_instance: export.provenance.source_instance.clone(),
        source_export_id: export.provenance.source_export_id.clone(),
        source_snapshot_sha256: export.provenance.snapshot_sha256.clone(),
        target_instance_uuid,
        target_snapshot_sha256,
        backup_instance_uuid,
        backup_sha256,
    })
}

pub async fn target_instance_uuid(
    connection: &mut SqliteConnection,
) -> Result<String, MigrationPreflightError> {
    let rows = sqlx::query("SELECT instance_uuid FROM opendesk_instance")
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| MigrationPreflightError::Database)?;
    match rows.as_slice() {
        [] => Err(MigrationPreflightError::TargetUninitialized),
        [row] => row
            .try_get("instance_uuid")
            .map_err(|_| MigrationPreflightError::Database),
        _ => Err(MigrationPreflightError::TargetIdentityAmbiguous),
    }
}

pub async fn logical_target_snapshot_digest(
    connection: &mut SqliteConnection,
) -> Result<String, MigrationPreflightError> {
    const SNAPSHOT_QUERIES: [(&str, &str); 12] = [
        ("users", "SELECT json_group_array(value) FROM (SELECT json_object('user_uuid', user_uuid, 'username', username, 'role', role, 'activation_state', activation_state) AS value FROM users ORDER BY user_uuid)"),
        ("sites", "SELECT json_group_array(value) FROM (SELECT json_object('site_uuid', site_uuid, 'name', name) AS value FROM sites ORDER BY site_uuid)"),
        ("devices", "SELECT json_group_array(value) FROM (SELECT json_object('device_uuid', device_uuid, 'rustdesk_id', rustdesk_id, 'alias', alias, 'hostname', hostname, 'site_uuid', site_uuid, 'owner', owner, 'notes', notes, 'archived', archived) AS value FROM devices ORDER BY device_uuid)"),
        ("access_groups", "SELECT json_group_array(value) FROM (SELECT json_object('access_group_uuid', access_group_uuid, 'name', name) AS value FROM access_groups ORDER BY access_group_uuid)"),
        ("access_group_memberships", "SELECT json_group_array(value) FROM (SELECT json_object('access_group_uuid', access_group_uuid, 'user_uuid', user_uuid) AS value FROM access_group_memberships ORDER BY access_group_uuid, user_uuid)"),
        ("device_visibility_grants", "SELECT json_group_array(value) FROM (SELECT json_object('access_group_uuid', access_group_uuid, 'device_uuid', device_uuid) AS value FROM device_visibility_grants ORDER BY access_group_uuid, device_uuid)"),
        ("user_device_visibility_grants", "SELECT json_group_array(value) FROM (SELECT json_object('user_uuid', user_uuid, 'device_uuid', device_uuid) AS value FROM user_device_visibility_grants ORDER BY user_uuid, device_uuid)"),
        ("address_books", "SELECT json_group_array(value) FROM (SELECT json_object('address_book_uuid', address_book_uuid, 'owner_user_uuid', owner_user_uuid, 'name', name, 'book_kind', book_kind) AS value FROM address_books ORDER BY address_book_uuid)"),
        ("address_book_access_rules", "SELECT json_group_array(value) FROM (SELECT json_object('address_book_uuid', address_book_uuid, 'principal_type', principal_type, 'principal_uuid', principal_uuid, 'permission', permission) AS value FROM address_book_access_rules ORDER BY address_book_uuid, principal_type, principal_uuid)"),
        ("address_book_tags", "SELECT json_group_array(value) FROM (SELECT json_object('address_book_uuid', address_book_uuid, 'name', name, 'color', color) AS value FROM address_book_tags ORDER BY address_book_uuid, name)"),
        ("address_book_entries", "SELECT json_group_array(value) FROM (SELECT json_object('address_book_entry_uuid', address_book_entry_uuid, 'address_book_uuid', address_book_uuid, 'device_uuid', device_uuid, 'alias', alias, 'notes', notes, 'position', position) AS value FROM address_book_entries ORDER BY address_book_entry_uuid)"),
        ("address_book_entry_tags", "SELECT json_group_array(value) FROM (SELECT json_object('address_book_entry_uuid', address_book_entry_uuid, 'address_book_uuid', address_book_uuid, 'tag_name', tag_name) AS value FROM address_book_entry_tags ORDER BY address_book_entry_uuid, tag_name)"),
    ];
    let mut digest = Sha256::new();
    for (name, query) in SNAPSHOT_QUERIES {
        let value: Option<String> = sqlx::query_scalar(query)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| MigrationPreflightError::Database)?;
        digest.update(name.as_bytes());
        digest.update(b"\n");
        digest.update(value.unwrap_or_else(|| "[]".to_string()).as_bytes());
        digest.update(b"\n");
    }
    Ok(hex::encode(digest.finalize()))
}

pub async fn instance_uuid_from_database(
    database: &Path,
) -> Result<String, MigrationPreflightError> {
    let mut connection = open_immutable_preflight_connection(database).await?;
    let instance_uuid = target_instance_uuid(&mut connection).await?;
    connection
        .close()
        .await
        .map_err(|_| MigrationPreflightError::Database)?;
    Ok(instance_uuid)
}

fn open_snapshot_file(database: &Path) -> Result<std::fs::File, MigrationPreflightError> {
    if database.starts_with("/proc/self/fd") {
        return std::fs::File::open(database).map_err(|_| MigrationPreflightError::InvalidTarget);
    }
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x20000 | 0x80000)
        .open(database)
        .map_err(|_| MigrationPreflightError::InvalidTarget)
}

pub fn snapshot_digest(database: &Path) -> Result<String, MigrationPreflightError> {
    let file = open_snapshot_file(database)?;
    let metadata = file
        .metadata()
        .map_err(|_| MigrationPreflightError::InvalidTarget)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || metadata.nlink() != 1 {
        return Err(MigrationPreflightError::InvalidTarget);
    }
    let wal = database.with_file_name(format!(
        "{}-wal",
        database
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(MigrationPreflightError::InvalidTarget)?
    ));
    if !database.starts_with("/proc/self/fd") {
        if let Ok(metadata) = fs::symlink_metadata(wal) {
            if !metadata.file_type().is_file() || metadata.len() != 0 {
                return Err(MigrationPreflightError::ActiveWal);
            }
        }
    }
    let mut reader = std::io::BufReader::new(file);
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| MigrationPreflightError::InvalidTarget)?;
    Ok(sha256_digest(&bytes))
}

pub async fn open_immutable_preflight_connection(
    database: &Path,
) -> Result<SqliteConnection, MigrationPreflightError> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(database)
        .read_only(true)
        .immutable(true)
        .create_if_missing(false);
    SqliteConnection::connect_with(&options)
        .await
        .map_err(|_| MigrationPreflightError::Database)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path(suffix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "opendesk-migration-preflight-{}-{suffix}",
            uuid::Uuid::new_v4()
        ))
    }

    #[tokio::test]
    async fn logical_digest_tracks_apply_surface_but_not_sessions() {
        let mut connection = SqliteConnection::connect("sqlite::memory:")
            .await
            .expect("connect");
        sqlx::query("CREATE TABLE users (user_uuid TEXT, username TEXT, password_hash TEXT, role TEXT, activation_state TEXT, created_at TEXT, updated_at TEXT)").execute(&mut connection).await.expect("users");
        sqlx::query(
            "CREATE TABLE sites (site_uuid TEXT, name TEXT, created_at TEXT, updated_at TEXT)",
        )
        .execute(&mut connection)
        .await
        .expect("sites");
        sqlx::query("CREATE TABLE devices (device_uuid TEXT, rustdesk_id TEXT, alias TEXT, hostname TEXT, os_family TEXT, os_version TEXT, architecture TEXT, rustdesk_version TEXT, site_uuid TEXT, owner TEXT, notes TEXT, last_checkin_at TEXT, last_lan_ip TEXT, last_wan_ip TEXT, archived INTEGER, created_at TEXT, updated_at TEXT)").execute(&mut connection).await.expect("devices");
        for table in ["access_groups (access_group_uuid TEXT, name TEXT)", "access_group_memberships (access_group_uuid TEXT, user_uuid TEXT)", "device_visibility_grants (access_group_uuid TEXT, device_uuid TEXT)", "user_device_visibility_grants (user_uuid TEXT, device_uuid TEXT)", "address_books (address_book_uuid TEXT, owner_user_uuid TEXT, name TEXT, book_kind TEXT)", "address_book_access_rules (address_book_uuid TEXT, principal_type TEXT, principal_uuid TEXT, permission TEXT)", "address_book_tags (address_book_uuid TEXT, name TEXT, color INTEGER)", "address_book_entries (address_book_entry_uuid TEXT, address_book_uuid TEXT, device_uuid TEXT, alias TEXT, notes TEXT, position INTEGER)", "address_book_entry_tags (address_book_entry_uuid TEXT, address_book_uuid TEXT, tag_name TEXT)"] {
            sqlx::query(&format!("CREATE TABLE {table}")).execute(&mut connection).await.expect("table");
        }
        let initial = logical_target_snapshot_digest(&mut connection)
            .await
            .expect("initial");
        sqlx::query("CREATE TABLE sessions (session_uuid TEXT)")
            .execute(&mut connection)
            .await
            .expect("sessions");
        sqlx::query("INSERT INTO sessions VALUES ('runtime-only')")
            .execute(&mut connection)
            .await
            .expect("session");
        assert_eq!(
            initial,
            logical_target_snapshot_digest(&mut connection)
                .await
                .expect("same")
        );
        sqlx::query(
            "INSERT INTO users VALUES ('u', 'user', 'hash', 'admin', 'active', 'now', 'now')",
        )
        .execute(&mut connection)
        .await
        .expect("user");
        let after_user = logical_target_snapshot_digest(&mut connection)
            .await
            .expect("user digest");
        assert_ne!(initial, after_user);
        sqlx::query("ALTER TABLE devices ADD COLUMN runtime_telemetry TEXT")
            .execute(&mut connection)
            .await
            .expect("telemetry");
        sqlx::query("UPDATE devices SET runtime_telemetry = 'changed'")
            .execute(&mut connection)
            .await
            .expect("telemetry update");
        assert_eq!(
            after_user,
            logical_target_snapshot_digest(&mut connection)
                .await
                .expect("telemetry ignored")
        );
    }

    #[test]
    fn snapshot_digest_rejects_missing_and_active_wal_targets() {
        let database = temporary_path("database.sqlite");
        assert_eq!(
            snapshot_digest(&database),
            Err(MigrationPreflightError::InvalidTarget)
        );
        std::fs::write(&database, b"sqlite snapshot").expect("write target");
        let wal = std::path::PathBuf::from(format!("{}-wal", database.display()));
        std::fs::write(&wal, b"active WAL").expect("write wal");
        assert_eq!(
            snapshot_digest(&database),
            Err(MigrationPreflightError::ActiveWal)
        );
        std::fs::remove_file(wal).expect("remove wal");
        assert_eq!(snapshot_digest(&database).expect("digest").len(), 64);
        std::fs::remove_file(database).expect("remove target");
    }
}
