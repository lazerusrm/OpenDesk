//! Locked verification gate for a future staging migration apply.
//!
//! Acquiring the guard makes no data changes. It opens the existing target,
//! acquires `BEGIN IMMEDIATE`, then verifies the signed plan and canonical target
//! state from that exact locked connection.

use std::{
    fs::{self, File},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawFd,
    },
    path::Path,
    str::FromStr,
};

unsafe extern "C" {
    fn geteuid() -> u32;
}

use sqlx::{Connection, SqliteConnection};
use thiserror::Error;
use time::OffsetDateTime;

use super::{
    migration_apply_plan::{verify_plan_signature, MigrationApplyPlan, MigrationApplyPlanError},
    migration_preflight::{
        instance_uuid_from_database, logical_target_snapshot_digest, target_instance_uuid,
        MigrationPreflight, MigrationPreflightError,
    },
};

#[derive(Debug, Error)]
pub enum MigrationApplyGuardError {
    #[error("migration apply guard database error")]
    Database(#[from] sqlx::Error),
    #[error("migration apply guard approval failed")]
    Approval(#[from] MigrationApplyPlanError),
    #[error("migration apply guard preflight failed")]
    Preflight(#[from] MigrationPreflightError),
    #[error("migration apply guard input does not match preflight")]
    InputMismatch,
    #[error("migration apply guard backup changed")]
    BackupChanged,
    #[error("migration apply guard target changed")]
    TargetChanged,
    #[error("migration apply guard target identity changed")]
    TargetIdentityChanged,
    #[error("migration apply guard target is not explicitly marked staging")]
    TargetNotStaging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    size: u64,
    uid: u32,
    mode: u32,
    nlink: u64,
}

fn open_verified(
    path: &Path,
    writable: bool,
) -> Result<(File, FileIdentity), MigrationApplyGuardError> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    if writable {
        options.write(true);
    }
    let file = options
        .custom_flags(0x20000 | 0x80000)
        .open(path)
        .map_err(|_| MigrationApplyGuardError::TargetChanged)?;
    let metadata = file
        .metadata()
        .map_err(|_| MigrationApplyGuardError::TargetChanged)?;
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.len() == 0
        || metadata.uid() != unsafe { geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(MigrationApplyGuardError::TargetChanged);
    }
    Ok((
        file,
        FileIdentity {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.len(),
            uid: metadata.uid(),
            mode: metadata.mode(),
            nlink: metadata.nlink(),
        },
    ))
}

fn identity(file: &File) -> Result<FileIdentity, MigrationApplyGuardError> {
    let metadata = file
        .metadata()
        .map_err(|_| MigrationApplyGuardError::TargetChanged)?;
    Ok(FileIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
        size: metadata.len(),
        uid: metadata.uid(),
        mode: metadata.mode(),
        nlink: metadata.nlink(),
    })
}

fn digest_file(file: &File) -> Result<String, MigrationApplyGuardError> {
    use std::io::{Read, Seek, SeekFrom};
    let mut bytes = Vec::new();
    let cloned = file
        .try_clone()
        .map_err(|_| MigrationApplyGuardError::BackupChanged)?;
    let mut reader = std::io::BufReader::new(cloned);
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|_| MigrationApplyGuardError::BackupChanged)?;
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| MigrationApplyGuardError::BackupChanged)?;
    Ok(crate::domain::migration_contract::sha256_digest(&bytes))
}

fn descriptor_path(file: &File) -> String {
    format!("/proc/self/fd/{}", file.as_raw_fd())
}

#[derive(Debug)]
pub struct StagingApplyGuard {
    connection: SqliteConnection,
    database_file: File,
    database_identity: FileIdentity,
    backup_file: File,
    backup_identity: FileIdentity,
}

impl StagingApplyGuard {
    pub async fn acquire(
        database: &Path,
        backup: &Path,
        input: &[u8],
        preflight: &MigrationPreflight,
        plan: &MigrationApplyPlan,
        public_key: &[u8],
        signature: &[u8],
        now: OffsetDateTime,
    ) -> Result<Self, MigrationApplyGuardError> {
        if crate::domain::migration_contract::sha256_digest(input) != preflight.input_sha256 {
            return Err(MigrationApplyGuardError::InputMismatch);
        }
        plan.validate_against_preflight(preflight, now)?;
        verify_plan_signature(plan, public_key, signature)?;
        let (backup_file, backup_identity) = open_verified(backup, false)?;
        if digest_file(&backup_file)? != preflight.backup_sha256 {
            return Err(MigrationApplyGuardError::BackupChanged);
        }
        let backup_uri = format!("/proc/self/fd/{}", backup_file.as_raw_fd());
        if instance_uuid_from_database(Path::new(&backup_uri)).await?
            != preflight.backup_instance_uuid
        {
            return Err(MigrationApplyGuardError::BackupChanged);
        }
        if digest_file(&backup_file)? != preflight.backup_sha256 {
            return Err(MigrationApplyGuardError::BackupChanged);
        }

        let (database_file, database_identity) = open_verified(database, true)?;
        let database_uri = format!("file:/proc/self/fd/{}?mode=rw", database_file.as_raw_fd());
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&database_uri)
            .map_err(|_| MigrationApplyGuardError::TargetChanged)?
            .create_if_missing(false);
        let mut connection = SqliteConnection::connect_with(&options).await?;
        if identity(&database_file).map_err(|_| MigrationApplyGuardError::TargetChanged)?
            != database_identity
        {
            return Err(MigrationApplyGuardError::TargetChanged);
        }
        if identity(&backup_file).map_err(|_| MigrationApplyGuardError::BackupChanged)?
            != backup_identity
        {
            return Err(MigrationApplyGuardError::BackupChanged);
        }
        if let Err(error) = sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut connection)
            .await
        {
            connection.close().await.ok();
            return Err(MigrationApplyGuardError::Database(error));
        }
        if identity(&database_file)? != database_identity {
            sqlx::query("ROLLBACK").execute(&mut connection).await.ok();
            connection.close().await.ok();
            return Err(MigrationApplyGuardError::TargetChanged);
        }
        let validation = Self::validate_locked_target(&mut connection, preflight).await;
        if let Err(error) = validation {
            sqlx::query("ROLLBACK").execute(&mut connection).await.ok();
            connection.close().await.ok();
            return Err(error);
        }
        Ok(Self {
            connection,
            database_file,
            database_identity,
            backup_file,
            backup_identity,
        })
    }

    async fn validate_locked_target(
        connection: &mut SqliteConnection,
        preflight: &MigrationPreflight,
    ) -> Result<(), MigrationApplyGuardError> {
        if logical_target_snapshot_digest(connection).await? != preflight.target_snapshot_sha256 {
            return Err(MigrationApplyGuardError::TargetChanged);
        }
        if target_instance_uuid(connection).await? != preflight.target_instance_uuid {
            return Err(MigrationApplyGuardError::TargetIdentityChanged);
        }
        let staging_marker: Option<(String,)> = sqlx::query_as(
            "SELECT instance_uuid FROM migration_staging_targets WHERE instance_uuid = ?",
        )
        .bind(&preflight.target_instance_uuid)
        .fetch_optional(&mut *connection)
        .await?;
        if staging_marker.is_none() {
            return Err(MigrationApplyGuardError::TargetNotStaging);
        }
        Ok(())
    }

    pub(crate) fn connection(&mut self) -> &mut SqliteConnection {
        &mut self.connection
    }

    pub(crate) async fn commit(mut self) -> Result<(), MigrationApplyGuardError> {
        sqlx::query("COMMIT").execute(&mut self.connection).await?;
        self.connection.close().await?;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), MigrationApplyGuardError> {
        sqlx::query("ROLLBACK")
            .execute(&mut self.connection)
            .await?;
        self.connection.close().await?;
        Ok(())
    }
}
