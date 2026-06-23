use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::device::{normalize_device_draft, Device, DeviceDraft};
use crate::time_format::format_timestamp;

type DeviceRow = (
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
    Option<String>,
    i64,
);

const DEVICE_SELECT: &str = "SELECT device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, last_checkin_at, archived";

fn row_to_device(row: DeviceRow) -> Device {
    Device {
        device_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
        rustdesk_id: row.1,
        alias: row.2,
        hostname: row.3,
        os_family: row.4,
        os_version: row.5,
        architecture: row.6,
        rustdesk_version: row.7,
        site_uuid: row.8.and_then(|value| Uuid::parse_str(&value).ok()),
        owner: row.9,
        notes: row.10,
        last_checkin_at: row.11,
        archived: row.12 != 0,
    }
}

pub async fn list_devices(pool: &SqlitePool) -> Result<Vec<Device>, sqlx::Error> {
    let rows = sqlx::query_as::<_, DeviceRow>(&format!("{DEVICE_SELECT} FROM devices ORDER BY alias ASC"))
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(row_to_device).collect())
}

pub async fn find_device_by_uuid(
    pool: &SqlitePool,
    device_uuid: Uuid,
) -> Result<Option<Device>, sqlx::Error> {
    let row = sqlx::query_as::<_, DeviceRow>(&format!("{DEVICE_SELECT} FROM devices WHERE device_uuid = ?"))
        .bind(device_uuid.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_device))
}

pub async fn find_device_by_rustdesk_id(
    pool: &SqlitePool,
    rustdesk_id: &str,
) -> Result<Option<Device>, sqlx::Error> {
    let row = sqlx::query_as::<_, DeviceRow>(&format!("{DEVICE_SELECT} FROM devices WHERE rustdesk_id = ?"))
        .bind(rustdesk_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_device))
}

pub async fn find_device_by_hostname(
    pool: &SqlitePool,
    hostname: &str,
) -> Result<Option<Device>, sqlx::Error> {
    let row = sqlx::query_as::<_, DeviceRow>(&format!("{DEVICE_SELECT} FROM devices WHERE hostname = ?"))
        .bind(hostname)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_device))
}

pub async fn create_device(pool: &SqlitePool, draft: &DeviceDraft) -> Result<Device, sqlx::Error> {
    let draft = normalize_device_draft(draft.clone());
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
    .bind(&draft.alias)
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
    let draft = normalize_device_draft(draft.clone());
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "UPDATE devices SET
            rustdesk_id = ?, alias = ?, hostname = ?, os_family = ?, os_version = ?,
            architecture = ?, rustdesk_version = ?, site_uuid = ?, owner = ?, notes = ?,
            updated_at = ?
         WHERE device_uuid = ?",
    )
    .bind(draft.rustdesk_id.as_deref())
    .bind(&draft.alias)
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
    let draft = normalize_device_draft(draft.clone());
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