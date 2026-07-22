//! Immutable preflight binding for a staged migration apply.
//!
//! This module has no write path. It binds the exact source bytes, a verified
//! backup digest, and an exact WAL-free target SQLite snapshot to one initialized
//! OpenDesk instance. A later apply must recheck these values while holding its
//! write lock.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
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
    backup_sha256: &str,
) -> Result<MigrationPreflight, MigrationPreflightError> {
    validate_digest(backup_sha256)?;
    let target_snapshot_sha256 = snapshot_digest(database)?;
    let mut connection = open_immutable_preflight_connection(database).await?;
    let target_instance_uuid = target_instance_uuid(&mut connection).await?;
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
        backup_sha256: backup_sha256.to_string(),
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

pub fn snapshot_digest(database: &Path) -> Result<String, MigrationPreflightError> {
    let metadata = fs::symlink_metadata(database).map_err(|_| MigrationPreflightError::InvalidTarget)?;
    if !metadata.file_type().is_file() || metadata.len() == 0 {
        return Err(MigrationPreflightError::InvalidTarget);
    }
    let wal = database.with_file_name(format!(
        "{}-wal",
        database.file_name().and_then(|value| value.to_str()).ok_or(MigrationPreflightError::InvalidTarget)?
    ));
    if let Ok(metadata) = fs::symlink_metadata(wal) {
        if !metadata.file_type().is_file() || metadata.len() != 0 {
            return Err(MigrationPreflightError::ActiveWal);
        }
    }
    let bytes = fs::read(database).map_err(|_| MigrationPreflightError::InvalidTarget)?;
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

fn validate_digest(value: &str) -> Result<(), MigrationPreflightError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(MigrationPreflightError::InvalidValue);
    }
    Ok(())
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

    #[test]
    fn snapshot_digest_rejects_missing_and_active_wal_targets() {
        let database = temporary_path("database.sqlite");
        assert_eq!(snapshot_digest(&database), Err(MigrationPreflightError::InvalidTarget));
        std::fs::write(&database, b"sqlite snapshot").expect("write target");
        let wal = std::path::PathBuf::from(format!("{}-wal", database.display()));
        std::fs::write(&wal, b"active WAL").expect("write wal");
        assert_eq!(snapshot_digest(&database), Err(MigrationPreflightError::ActiveWal));
        std::fs::remove_file(wal).expect("remove wal");
        assert_eq!(snapshot_digest(&database).expect("digest").len(), 64);
        std::fs::remove_file(database).expect("remove target");
    }
}
