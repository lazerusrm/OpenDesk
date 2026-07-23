use std::{
    fs::{self, File},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawFd,
    },
    path::Path,
    str::FromStr,
};

use sqlx::{Connection, SqliteConnection};
use thiserror::Error;
use time::OffsetDateTime;

use super::migration_apply::MigrationApplyError;
use crate::{
    domain::{
        migration_apply_plan::MigrationApplyPlan, migration_contract::SanitizedMigrationExport,
        migration_preflight::MigrationPreflight,
    },
    time_format::format_timestamp,
};

const MAX_CREDENTIAL_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;

unsafe extern "C" {
    fn geteuid() -> u32;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialAttachmentError {
    #[error("credential attachment database is invalid")]
    InvalidDatabase,
    #[error("credential attachment digest is invalid")]
    InvalidDigest,
    #[error("credential attachment artifact is invalid")]
    InvalidArtifact,
    #[error("credential attachment migration binding does not match")]
    BindingMismatch,
    #[error("credential attachment was already attempted")]
    AlreadyAttached,
    #[error("credential attachment requires disabled bound users")]
    ActiveUser,
    #[error("credential attachment database operation failed")]
    Database,
}

pub async fn attach_protected_credentials(
    database: &Path,
    artifact_bytes: &[u8],
    expected_sha256: &str,
    now: OffsetDateTime,
) -> Result<(), CredentialAttachmentError> {
    validate_expected_digest(expected_sha256)?;
    if artifact_bytes.is_empty() || artifact_bytes.len() > MAX_CREDENTIAL_ARTIFACT_BYTES {
        return Err(CredentialAttachmentError::InvalidArtifact);
    }
    let artifact_sha256 = crate::domain::migration_contract::sha256_digest(artifact_bytes);
    if artifact_sha256 != expected_sha256 {
        return Err(CredentialAttachmentError::InvalidDigest);
    }
    let artifact = crate::domain::migration_credentials::parse_artifact(artifact_bytes)
        .map_err(|_| CredentialAttachmentError::InvalidArtifact)?;
    let database_file = open_attachment_database(database)?;
    let database_uri = format!("file:/proc/self/fd/{}?mode=rw", database_file.as_raw_fd());
    let options = sqlx::sqlite::SqliteConnectOptions::from_str(&database_uri)
        .map_err(|_| CredentialAttachmentError::InvalidDatabase)?
        .create_if_missing(false);
    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(|_| CredentialAttachmentError::Database)?;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut connection)
        .await
        .map_err(|_| CredentialAttachmentError::Database)?;
    let result = attach_locked(
        &mut connection,
        &artifact,
        &artifact_sha256,
        format_timestamp(now),
    )
    .await;
    match result {
        Ok(()) => {
            sqlx::query("COMMIT")
                .execute(&mut connection)
                .await
                .map_err(|_| CredentialAttachmentError::Database)?;
            connection
                .close()
                .await
                .map_err(|_| CredentialAttachmentError::Database)?;
            Ok(())
        }
        Err(error) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut connection).await;
            let _ = connection.close().await;
            Err(error)
        }
    }
}

fn validate_expected_digest(value: &str) -> Result<(), CredentialAttachmentError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(CredentialAttachmentError::InvalidDigest);
    }
    Ok(())
}

fn open_attachment_database(path: &Path) -> Result<File, CredentialAttachmentError> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(0x20000 | 0x80000)
        .open(path)
        .map_err(|_| CredentialAttachmentError::InvalidDatabase)?;
    let metadata = file
        .metadata()
        .map_err(|_| CredentialAttachmentError::InvalidDatabase)?;
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.len() == 0
        || metadata.uid() != unsafe { geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(CredentialAttachmentError::InvalidDatabase);
    }
    Ok(file)
}

async fn attach_locked(
    connection: &mut SqliteConnection,
    artifact: &crate::domain::migration_credentials::ProtectedCredentialArtifact,
    artifact_sha256: &str,
    attached_at: String,
) -> Result<(), CredentialAttachmentError> {
    let run_id = artifact.run_id.to_string();
    let run: Option<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT source_system, source_instance, source_export_id, target_instance_uuid, input_sha256, status FROM migration_runs WHERE run_id = ?",
    )
    .bind(&run_id)
    .fetch_optional(&mut *connection)
    .await
    .map_err(|_| CredentialAttachmentError::Database)?;
    let Some((
        source_system,
        source_instance,
        source_export_id,
        target_instance,
        input_sha256,
        status,
    )) = run
    else {
        return Err(CredentialAttachmentError::BindingMismatch);
    };
    if source_system != artifact.source_system
        || source_instance != artifact.source_instance
        || source_export_id != artifact.source_export_id
        || target_instance != artifact.target_instance_uuid.to_string()
        || input_sha256 != artifact.input_sha256
        || status != "applied"
    {
        return Err(CredentialAttachmentError::BindingMismatch);
    }
    let current_instance: Option<String> =
        sqlx::query_scalar("SELECT instance_uuid FROM opendesk_instance LIMIT 1")
            .fetch_optional(&mut *connection)
            .await
            .map_err(|_| CredentialAttachmentError::Database)?;
    if current_instance.as_deref() != Some(target_instance.as_str()) {
        return Err(CredentialAttachmentError::BindingMismatch);
    }
    let receipt: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM migration_credential_attachment_receipts WHERE run_id = ?",
    )
    .bind(&run_id)
    .fetch_optional(&mut *connection)
    .await
    .map_err(|_| CredentialAttachmentError::Database)?;
    if receipt.is_some() {
        return Err(CredentialAttachmentError::AlreadyAttached);
    }
    let bindings: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT source_system, source_instance, source_id, target_uuid FROM migration_source_bindings WHERE run_id = ? AND entity_kind = 'user' ORDER BY source_id",
    )
    .bind(&run_id)
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| CredentialAttachmentError::Database)?;
    let artifact_sources: std::collections::HashSet<&str> = artifact
        .records
        .iter()
        .map(|record| record.source_user_id.as_str())
        .collect();
    let binding_sources: std::collections::HashSet<&str> = bindings
        .iter()
        .map(|(_, _, source_id, _)| source_id.as_str())
        .collect();
    let target_users: std::collections::HashSet<&str> = bindings
        .iter()
        .map(|(_, _, _, target_uuid)| target_uuid.as_str())
        .collect();
    if bindings
        .iter()
        .any(|(binding_system, binding_instance, _, _)| {
            binding_system != &source_system || binding_instance != &source_instance
        })
        || artifact_sources != binding_sources
        || bindings.len() != artifact.records.len()
        || target_users.len() != bindings.len()
    {
        return Err(CredentialAttachmentError::BindingMismatch);
    }
    let existing_for_run: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM migration_legacy_credentials WHERE run_id = ?")
            .bind(&run_id)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| CredentialAttachmentError::Database)?;
    if existing_for_run != 0 {
        return Err(CredentialAttachmentError::AlreadyAttached);
    }
    for (_, _, source_id, target_uuid) in &bindings {
        let state: Option<String> =
            sqlx::query_scalar("SELECT activation_state FROM users WHERE user_uuid = ?")
                .bind(target_uuid)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| CredentialAttachmentError::Database)?;
        if state.as_deref() != Some("disabled") {
            return Err(CredentialAttachmentError::ActiveUser);
        }
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM migration_legacy_credentials WHERE user_uuid = ?")
                .bind(target_uuid)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| CredentialAttachmentError::Database)?;
        if existing.is_some() {
            return Err(CredentialAttachmentError::AlreadyAttached);
        }
        let record = artifact
            .records
            .iter()
            .find(|record| record.source_user_id == *source_id)
            .ok_or(CredentialAttachmentError::BindingMismatch)?;
        sqlx::query("INSERT INTO migration_legacy_credentials (user_uuid, run_id, verifier_algorithm, verifier, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(target_uuid)
            .bind(&run_id)
            .bind(&record.verifier_algorithm)
            .bind(&record.verifier)
            .bind(&attached_at)
            .execute(&mut *connection)
            .await
            .map_err(|_| CredentialAttachmentError::Database)?;
    }
    for (_, _, _, target_uuid) in &bindings {
        let activated = sqlx::query(
            "UPDATE users SET password_hash = '', activation_state = 'active', updated_at = ? WHERE user_uuid = ? AND activation_state = 'disabled'",
        )
        .bind(&attached_at)
        .bind(target_uuid)
        .execute(&mut *connection)
        .await
        .map_err(|_| CredentialAttachmentError::Database)?;
        if activated.rows_affected() != 1 {
            return Err(CredentialAttachmentError::ActiveUser);
        }
    }
    sqlx::query("INSERT INTO migration_credential_attachment_receipts (run_id, artifact_sha256, attached_at, record_count) VALUES (?, ?, ?, ?)")
        .bind(&run_id)
        .bind(artifact_sha256)
        .bind(&attached_at)
        .bind(bindings.len() as i64)
        .execute(&mut *connection)
        .await
        .map_err(|_| CredentialAttachmentError::Database)?;
    Ok(())
}

pub(crate) async fn import_protected_credentials(
    connection: &mut sqlx::SqliteConnection,
    export: &SanitizedMigrationExport,
    preflight: &MigrationPreflight,
    plan: &MigrationApplyPlan,
    now: OffsetDateTime,
    credentials: Option<&[u8]>,
) -> Result<(), MigrationApplyError> {
    let bytes = match credentials {
        Some(value) => value,
        None if plan.credential_artifact_sha256.is_some() => {
            return Err(MigrationApplyError::CredentialArtifactRequired)
        }
        None => return Ok(()),
    };
    let artifact = crate::domain::migration_credentials::parse_artifact(bytes)?;
    let run_status: Option<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT source_system, source_instance, source_export_id, target_instance_uuid, status, plan_sha256 FROM migration_runs WHERE run_id = ?",
    ).bind(export.run.run_id.to_string()).fetch_optional(&mut *connection).await?;
    let Some((
        source_system,
        source_instance,
        source_export_id,
        target_instance_uuid,
        run_status,
        plan_sha256,
    )) = run_status
    else {
        return Err(MigrationApplyError::CredentialRunMismatch);
    };
    let expected_plan = crate::domain::migration_contract::sha256_digest(
        &plan
            .canonical_bytes()
            .map_err(|_| MigrationApplyError::CredentialBindingMismatch)?,
    );
    if source_system != export.provenance.source_system
        || source_instance != export.provenance.source_instance
        || source_export_id != export.provenance.source_export_id
        || target_instance_uuid != preflight.target_instance_uuid
        || run_status != "applied"
        || plan_sha256 != expected_plan
    {
        return Err(MigrationApplyError::CredentialRunMismatch);
    }
    let expected: std::collections::HashSet<&str> = export
        .users
        .iter()
        .map(|u| u.source_user_id.as_str())
        .collect();
    let actual: std::collections::HashSet<&str> = artifact
        .records
        .iter()
        .map(|r| r.source_user_id.as_str())
        .collect();
    if expected != actual || expected.len() != artifact.records.len() {
        return Err(MigrationApplyError::CredentialSetMismatch);
    }
    for record in &artifact.records {
        let target_uuid: Option<(String, String)> = sqlx::query_as("SELECT target_uuid, run_id FROM migration_source_bindings WHERE source_system = ? AND source_instance = ? AND entity_kind = 'user' AND source_id = ?").bind(&export.provenance.source_system).bind(&export.provenance.source_instance).bind(&record.source_user_id).fetch_optional(&mut *connection).await?;
        let Some((target_uuid, binding_run)) = target_uuid else {
            return Err(MigrationApplyError::CredentialBindingMissing);
        };
        if binding_run != export.run.run_id.to_string() {
            return Err(MigrationApplyError::CredentialBindingMismatch);
        }
        let inserted = sqlx::query("INSERT INTO migration_legacy_credentials (user_uuid, run_id, verifier_algorithm, verifier, created_at) VALUES (?, ?, ?, ?, ?)").bind(&target_uuid).bind(export.run.run_id.to_string()).bind(&record.verifier_algorithm).bind(&record.verifier).bind(format_timestamp(now)).execute(&mut *connection).await?;
        if inserted.rows_affected() != 1 {
            return Err(MigrationApplyError::CredentialBindingMismatch);
        }
        let activated = sqlx::query("UPDATE users SET password_hash = '', activation_state = 'active' WHERE user_uuid = ? AND activation_state = 'disabled'").bind(&target_uuid).execute(&mut *connection).await?;
        if activated.rows_affected() != 1 {
            return Err(MigrationApplyError::CredentialBindingMismatch);
        }
    }
    Ok(())
}
