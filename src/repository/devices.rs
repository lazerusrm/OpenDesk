use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::device::{Device, DeviceDraft};
use crate::time_format::format_timestamp;

fn row_to_device(
    device_uuid: String,
    rustdesk_id: Option<String>,
    alias: String,
    hostname: Option<String>,
    os_family: Option<String>,
    os_version: Option<String>,
    architecture: Option<String>,
    rustdesk_version: Option<String>,
    site_uuid: Option<String>,
    owner: Option<String>,
    notes: Option<String>,
    archived: i64,
) -> Device {
    Device {
        device_uuid: Uuid::parse_str(&device_uuid).expect("stored uuid"),
        rustdesk_id,
        alias,
        hostname,
        os_family,
        os_version,
        architecture,
        rustdesk_version,
        site_uuid: site_uuid.and_then(|value| Uuid::parse_str(&value).ok()),
        owner,
        notes,
        archived: archived != 0,
    }
}

pub async fn list_devices(pool: &SqlitePool) -> Result<Vec<Device>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    )>(
        "SELECT device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, archived
         FROM devices ORDER BY alias ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row_to_device(row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11))
        .collect())
}

pub async fn find_device_by_uuid(
    pool: &SqlitePool,
    device_uuid: Uuid,
) -> Result<Option<Device>, sqlx::Error> {
    let row = sqlx::query_as::<_, (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    )>(
        "SELECT device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, archived
         FROM devices WHERE device_uuid = ?",
    )
    .bind(device_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| row_to_device(row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11)))
}

pub async fn find_device_by_rustdesk_id(
    pool: &SqlitePool,
    rustdesk_id: &str,
) -> Result<Option<Device>, sqlx::Error> {
    let row = sqlx::query_as::<_, (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    )>(
        "SELECT device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, archived
         FROM devices WHERE rustdesk_id = ?",
    )
    .bind(rustdesk_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| row_to_device(row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9, row.10, row.11)))
}

pub async fn create_device(pool: &SqlitePool, draft: &DeviceDraft) -> Result<Device, sqlx::Error> {
    let device_uuid = Uuid::new_v4();
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO devices (
            device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
            rustdesk_version, site_uuid, owner, notes, archived, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(device_uuid.to_string())
    .bind(draft.rustdesk_id.as_deref())
    .bind(draft.alias.trim())
    .bind(draft.hostname.as_deref())
    .bind(draft.os_family.as_deref())
    .bind(draft.os_version.as_deref())
    .bind(draft.architecture.as_deref())
    .bind(draft.rustdesk_version.as_deref())
    .bind(draft.site_uuid.map(|value| value.to_string()))
    .bind(draft.owner.as_deref())
    .bind(draft.notes.as_deref())
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    find_device_by_uuid(pool, device_uuid)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn update_device(
    pool: &SqlitePool,
    device_uuid: Uuid,
    draft: &DeviceDraft,
) -> Result<Device, sqlx::Error> {
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "UPDATE devices SET
            rustdesk_id = ?, alias = ?, hostname = ?, os_family = ?, os_version = ?,
            architecture = ?, rustdesk_version = ?, site_uuid = ?, owner = ?, notes = ?,
            updated_at = ?
         WHERE device_uuid = ?",
    )
    .bind(draft.rustdesk_id.as_deref())
    .bind(draft.alias.trim())
    .bind(draft.hostname.as_deref())
    .bind(draft.os_family.as_deref())
    .bind(draft.os_version.as_deref())
    .bind(draft.architecture.as_deref())
    .bind(draft.rustdesk_version.as_deref())
    .bind(draft.site_uuid.map(|value| value.to_string()))
    .bind(draft.owner.as_deref())
    .bind(draft.notes.as_deref())
    .bind(&now)
    .bind(device_uuid.to_string())
    .execute(pool)
    .await?;
    find_device_by_uuid(pool, device_uuid)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn set_device_archived(
    pool: &SqlitePool,
    device_uuid: Uuid,
    archived: bool,
) -> Result<Device, sqlx::Error> {
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query("UPDATE devices SET archived = ?, updated_at = ? WHERE device_uuid = ?")
        .bind(if archived { 1 } else { 0 })
        .bind(&now)
        .bind(device_uuid.to_string())
        .execute(pool)
        .await?;
    find_device_by_uuid(pool, device_uuid)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn touch_device_checkin(
    pool: &SqlitePool,
    device_uuid: Uuid,
    draft: &DeviceDraft,
) -> Result<Device, sqlx::Error> {
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "UPDATE devices SET
            rustdesk_id = COALESCE(?, rustdesk_id),
            hostname = COALESCE(?, hostname),
            os_family = COALESCE(?, os_family),
            os_version = COALESCE(?, os_version),
            architecture = COALESCE(?, architecture),
            rustdesk_version = COALESCE(?, rustdesk_version),
            last_checkin_at = ?, updated_at = ?
         WHERE device_uuid = ?",
    )
    .bind(draft.rustdesk_id.as_deref())
    .bind(draft.hostname.as_deref())
    .bind(draft.os_family.as_deref())
    .bind(draft.os_version.as_deref())
    .bind(draft.architecture.as_deref())
    .bind(draft.rustdesk_version.as_deref())
    .bind(&now)
    .bind(&now)
    .bind(device_uuid.to_string())
    .execute(pool)
    .await?;
    find_device_by_uuid(pool, device_uuid)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}