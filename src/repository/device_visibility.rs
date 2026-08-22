use std::collections::HashSet;

use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::domain::device_visibility::{AccessGroupAccessGrant, UserDeviceVisibilityGrant};
use crate::domain::role::Role;

#[derive(Debug, Error)]
pub enum DeviceVisibilityRepositoryError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("duplicate UUID in replacement list: {0}")]
    DuplicateUuid(Uuid),
    #[error("an access group cannot grant access to itself")]
    SelfAccessGrant,
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

async fn user_is_admin(pool: &SqlitePool, user_uuid: Uuid) -> Result<bool, sqlx::Error> {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE user_uuid = ?")
        .bind(user_uuid.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(role.as_deref() == Some(Role::ADMIN))
}

pub async fn ensure_user_device_visibility_grant(
    pool: &SqlitePool,
    user_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO user_device_visibility_grants (user_uuid, device_uuid)
         VALUES (?, ?)",
    )
    .bind(user_uuid.to_string())
    .bind(device_uuid.to_string())
    .execute(pool)
    .await?;
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

pub async fn list_outgoing_access_group_access_grants(
    pool: &SqlitePool,
    incoming_access_group_uuid: Uuid,
) -> Result<Vec<AccessGroupAccessGrant>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT incoming_access_group_uuid, outgoing_access_group_uuid
         FROM access_group_access_grants
         WHERE incoming_access_group_uuid = ?
         ORDER BY outgoing_access_group_uuid ASC",
    )
    .bind(incoming_access_group_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(incoming, outgoing)| AccessGroupAccessGrant {
            incoming_access_group_uuid: parse_uuid(incoming),
            outgoing_access_group_uuid: parse_uuid(outgoing),
        })
        .collect())
}

pub async fn list_incoming_access_group_access_grants(
    pool: &SqlitePool,
    outgoing_access_group_uuid: Uuid,
) -> Result<Vec<AccessGroupAccessGrant>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT incoming_access_group_uuid, outgoing_access_group_uuid
         FROM access_group_access_grants
         WHERE outgoing_access_group_uuid = ?
         ORDER BY incoming_access_group_uuid ASC",
    )
    .bind(outgoing_access_group_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(incoming, outgoing)| AccessGroupAccessGrant {
            incoming_access_group_uuid: parse_uuid(incoming),
            outgoing_access_group_uuid: parse_uuid(outgoing),
        })
        .collect())
}

pub async fn replace_outgoing_access_group_access_grants(
    pool: &SqlitePool,
    incoming_access_group_uuid: Uuid,
    outgoing_access_group_uuids: &[Uuid],
) -> Result<(), DeviceVisibilityRepositoryError> {
    reject_duplicates(outgoing_access_group_uuids)?;
    if outgoing_access_group_uuids.contains(&incoming_access_group_uuid) {
        return Err(DeviceVisibilityRepositoryError::SelfAccessGrant);
    }
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM access_groups WHERE access_group_uuid = ? LIMIT 1")
        .bind(incoming_access_group_uuid.to_string())
        .fetch_one(&mut *tx)
        .await?;
    for outgoing in outgoing_access_group_uuids {
        sqlx::query("SELECT 1 FROM access_groups WHERE access_group_uuid = ? LIMIT 1")
            .bind(outgoing.to_string())
            .fetch_one(&mut *tx)
            .await?;
    }
    sqlx::query("DELETE FROM access_group_access_grants WHERE incoming_access_group_uuid = ?")
        .bind(incoming_access_group_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    for outgoing in outgoing_access_group_uuids {
        sqlx::query(
            "INSERT INTO access_group_access_grants
             (incoming_access_group_uuid, outgoing_access_group_uuid)
             VALUES (?, ?)",
        )
        .bind(incoming_access_group_uuid.to_string())
        .bind(outgoing.to_string())
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
    if user_is_admin(pool, user_uuid).await? {
        let exists: i64 =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE device_uuid = ?)")
                .bind(device_uuid.to_string())
                .fetch_one(pool)
                .await?;
        return Ok(exists != 0);
    }
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
        ) OR EXISTS (
            SELECT 1
            FROM access_group_memberships incoming_membership
            INNER JOIN access_group_access_grants group_access
                ON group_access.incoming_access_group_uuid = incoming_membership.access_group_uuid
            INNER JOIN device_visibility_grants outgoing_grant
                ON outgoing_grant.access_group_uuid = group_access.outgoing_access_group_uuid
            WHERE incoming_membership.user_uuid = ?
              AND outgoing_grant.device_uuid = ?
        )",
    )
    .bind(user_uuid.to_string())
    .bind(device_uuid.to_string())
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
    if user_is_admin(pool, user_uuid).await? {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT device_uuid FROM devices ORDER BY device_uuid ASC",
        )
        .fetch_all(pool)
        .await?;
        return Ok(rows
            .into_iter()
            .map(|(device_uuid,)| parse_uuid(device_uuid))
            .collect());
    }
    list_explicitly_granted_device_uuids_for_user(pool, user_uuid).await
}

pub async fn list_explicitly_granted_device_uuids_for_user(
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
             UNION
             SELECT outgoing_grant.device_uuid
             FROM access_group_memberships incoming_membership
             INNER JOIN access_group_access_grants group_access
                 ON group_access.incoming_access_group_uuid = incoming_membership.access_group_uuid
             INNER JOIN device_visibility_grants outgoing_grant
                 ON outgoing_grant.access_group_uuid = group_access.outgoing_access_group_uuid
             WHERE incoming_membership.user_uuid = ?
         )
         ORDER BY device_uuid ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(device_uuid,)| parse_uuid(device_uuid))
        .collect())
}
