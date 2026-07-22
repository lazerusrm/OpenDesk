use std::collections::HashSet;

use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::device_visibility::UserDeviceVisibilityGrant;

#[derive(Debug, Error)]
pub enum DeviceVisibilityRepositoryError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("duplicate UUID in replacement list: {0}")]
    DuplicateUuid(Uuid),
}

fn parse_uuid(value: String) -> Uuid {
    Uuid::parse_str(&value).expect("stored uuid")
}

fn reject_duplicates(values: &[Uuid]) -> Result<(), DeviceVisibilityRepositoryError> {
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        if !seen.insert(*value) {
            return Err(DeviceVisibilityRepositoryError::DuplicateUuid(*value));
        }
    }
    Ok(())
}

pub async fn list_user_device_visibility_grants(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<UserDeviceVisibilityGrant>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT user_uuid, device_uuid
         FROM user_device_visibility_grants
         WHERE user_uuid = ? ORDER BY device_uuid ASC",
    )
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(user, device)| UserDeviceVisibilityGrant {
            user_uuid: parse_uuid(user),
            device_uuid: parse_uuid(device),
        })
        .collect())
}

pub async fn replace_user_device_visibility_grants(
    pool: &SqlitePool,
    user_uuid: Uuid,
    device_uuids: &[Uuid],
) -> Result<(), DeviceVisibilityRepositoryError> {
    reject_duplicates(device_uuids)?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM users WHERE user_uuid = ? LIMIT 1")
        .bind(user_uuid.to_string())
        .fetch_one(&mut *tx)
        .await?;
    for device_uuid in device_uuids {
        sqlx::query("SELECT 1 FROM devices WHERE device_uuid = ? LIMIT 1")
            .bind(device_uuid.to_string())
            .fetch_one(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM user_device_visibility_grants WHERE user_uuid = ?")
        .bind(user_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    for device_uuid in device_uuids {
        sqlx::query(
            "INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)",
        )
        .bind(user_uuid.to_string())
        .bind(device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn is_device_visible_to_user(
    pool: &SqlitePool,
    user_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM user_device_visibility_grants
            WHERE user_uuid = ? AND device_uuid = ?
        ) OR EXISTS (
            SELECT 1
            FROM access_group_memberships membership
            INNER JOIN device_visibility_grants device_grant
                ON device_grant.access_group_uuid = membership.access_group_uuid
            WHERE membership.user_uuid = ? AND device_grant.device_uuid = ?
        )",
    )
    .bind(user_uuid.to_string())
    .bind(device_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(device_uuid.to_string())
    .fetch_one(pool)
    .await?;
    Ok(row.0 != 0)
}

pub async fn list_visible_device_uuids_for_user(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT device_uuid
         FROM (
             SELECT device_uuid
             FROM user_device_visibility_grants
             WHERE user_uuid = ?
             UNION
             SELECT device_grant.device_uuid
             FROM access_group_memberships membership
             INNER JOIN device_visibility_grants device_grant
                 ON device_grant.access_group_uuid = membership.access_group_uuid
             WHERE membership.user_uuid = ?
         )
         ORDER BY device_uuid ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(device_uuid,)| parse_uuid(device_uuid))
        .collect())
}
