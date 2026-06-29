use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::backup::{
    BackupDeviceTag, BackupDocument, BackupEnrollmentToken, BackupSensitivity, BackupUser,
    BACKUP_SCHEMA_VERSION,
};
use crate::time_format::format_timestamp;

use super::devices::list_devices;
use super::enrollment_tokens::list_enrollment_tokens;
use super::server_config::load_server_config;
use super::sites::list_sites;
use super::tags::list_device_tag_links;
use super::tags::list_tags;
use super::users::list_users;

#[derive(Debug, thiserror::Error)]
pub enum BackupRestoreError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("backup validation failed: {0}")]
    Validation(#[from] crate::domain::backup::BackupValidationError),
}

pub async fn export_backup_document(pool: &SqlitePool) -> Result<BackupDocument, sqlx::Error> {
    let sites = list_sites(pool).await?;
    let tags = list_tags(pool).await?;
    let devices = list_devices(pool).await?;
    let device_tags = list_device_tag_links(pool)
        .await?
        .into_iter()
        .map(|(device_uuid, tag_uuid)| BackupDeviceTag {
            device_uuid,
            tag_uuid,
        })
        .collect();
    let server_config = load_server_config(pool).await?;
    let enrollment_tokens = list_enrollment_tokens(pool)
        .await?
        .into_iter()
        .map(|token| BackupEnrollmentToken {
            enrollment_token_uuid: token.enrollment_token_uuid,
            token_hash: token.token_hash,
            label: token.label,
            site_uuid: token.site_uuid,
            expires_at: token.expires_at.map(format_timestamp),
            revoked_at: token.revoked_at.map(format_timestamp),
        })
        .collect();
    let users = list_users(pool)
        .await?
        .into_iter()
        .map(|user| BackupUser {
            user_uuid: user.user_uuid,
            username: user.username,
            password_hash: user.password_hash,
            role: user.role,
        })
        .collect();
    Ok(BackupDocument {
        schema_version: BACKUP_SCHEMA_VERSION,
        exported_at: format_timestamp(OffsetDateTime::now_utc()),
        sensitivity: BackupSensitivity::default(),
        sites,
        tags,
        devices,
        device_tags,
        server_config,
        enrollment_tokens,
        users,
    })
}

pub async fn restore_backup_document(
    pool: &SqlitePool,
    document: &BackupDocument,
) -> Result<(), BackupRestoreError> {
    crate::domain::backup::validate_backup_document(document)?;
    let now = format_timestamp(OffsetDateTime::now_utc());
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM device_tags").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM endpoint_checkins")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM devices").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM enrollment_tokens")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tags").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM sites").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM server_configs")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM users").execute(&mut *tx).await?;

    for site in &document.sites {
        sqlx::query(
            "INSERT INTO sites (site_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(site.site_uuid.to_string())
        .bind(&site.name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for tag in &document.tags {
        sqlx::query(
            "INSERT INTO tags (tag_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(tag.tag_uuid.to_string())
        .bind(&tag.name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for device in &document.devices {
        sqlx::query(
            "INSERT INTO devices (
                device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, last_checkin_at, archived,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(device.device_uuid.to_string())
        .bind(device.rustdesk_id.as_deref())
        .bind(&device.alias)
        .bind(device.hostname.as_deref())
        .bind(device.os_family.as_deref())
        .bind(device.os_version.as_deref())
        .bind(device.architecture.as_deref())
        .bind(device.rustdesk_version.as_deref())
        .bind(device.site_uuid.map(|value| value.to_string()))
        .bind(device.owner.as_deref())
        .bind(device.notes.as_deref())
        .bind(device.last_checkin_at.as_deref())
        .bind(if device.archived { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for link in &document.device_tags {
        sqlx::query("INSERT INTO device_tags (device_uuid, tag_uuid) VALUES (?, ?)")
            .bind(link.device_uuid.to_string())
            .bind(link.tag_uuid.to_string())
            .execute(&mut *tx)
            .await?;
    }
    if let Some(config) = &document.server_config {
        sqlx::query(
            "INSERT INTO server_configs (
                server_config_uuid, id_server, relay_server, api_server, public_key,
                updated_at, updated_by_user_uuid
             ) VALUES (?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&config.id_server)
        .bind(&config.relay_server)
        .bind(&config.api_server)
        .bind(&config.public_key)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for token in &document.enrollment_tokens {
        sqlx::query(
            "INSERT INTO enrollment_tokens (
                enrollment_token_uuid, token_hash, label, site_uuid, expires_at, revoked_at, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(token.enrollment_token_uuid.to_string())
        .bind(&token.token_hash)
        .bind(&token.label)
        .bind(token.site_uuid.map(|value| value.to_string()))
        .bind(token.expires_at.as_deref())
        .bind(token.revoked_at.as_deref())
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for user in &document.users {
        sqlx::query(
            "INSERT INTO users (user_uuid, username, password_hash, role, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user.user_uuid.to_string())
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

