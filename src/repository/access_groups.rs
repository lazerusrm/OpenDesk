use std::collections::HashSet;

use sqlx::{Sqlite, SqlitePool, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::access_group::{AccessGroup, AccessGroupValidationError};
use crate::domain::access_group_membership::AccessGroupMembership;
use crate::domain::device_visibility::DeviceVisibilityGrant;

#[derive(Debug, Error)]
pub enum AccessGroupRepositoryError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("access group validation failed: {0}")]
    Validation(#[from] AccessGroupValidationError),
    #[error("duplicate UUID in replacement list: {0}")]
    DuplicateUuid(Uuid),
}

fn parse_uuid(value: String) -> Uuid {
    Uuid::parse_str(&value).expect("stored uuid")
}

async fn require_access_group(
    tx: &mut Transaction<'_, Sqlite>,
    access_group_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1 FROM access_groups WHERE access_group_uuid = ? LIMIT 1")
        .bind(access_group_uuid.to_string())
        .fetch_one(&mut **tx)
        .await
        .map(|_| ())
}

async fn require_user(
    tx: &mut Transaction<'_, Sqlite>,
    user_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1 FROM users WHERE user_uuid = ? LIMIT 1")
        .bind(user_uuid.to_string())
        .fetch_one(&mut **tx)
        .await
        .map(|_| ())
}

async fn require_device(
    tx: &mut Transaction<'_, Sqlite>,
    device_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1 FROM devices WHERE device_uuid = ? LIMIT 1")
        .bind(device_uuid.to_string())
        .fetch_one(&mut **tx)
        .await
        .map(|_| ())
}

fn reject_duplicates(values: &[Uuid]) -> Result<(), AccessGroupRepositoryError> {
    let mut seen = HashSet::with_capacity(values.len());
    for value in values {
        if !seen.insert(*value) {
            return Err(AccessGroupRepositoryError::DuplicateUuid(*value));
        }
    }
    Ok(())
}

pub async fn list_access_groups(pool: &SqlitePool) -> Result<Vec<AccessGroup>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, name FROM access_groups ORDER BY name ASC, access_group_uuid ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(access_group_uuid, name)| AccessGroup {
            access_group_uuid: parse_uuid(access_group_uuid),
            name,
        })
        .collect())
}

pub async fn find_access_group_by_uuid(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
) -> Result<Option<AccessGroup>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, name FROM access_groups WHERE access_group_uuid = ?",
    )
    .bind(access_group_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(access_group_uuid, name)| AccessGroup {
        access_group_uuid: parse_uuid(access_group_uuid),
        name,
    }))
}

pub async fn create_access_group(
    pool: &SqlitePool,
    name: &str,
) -> Result<AccessGroup, AccessGroupRepositoryError> {
    let group = AccessGroup::new(Uuid::new_v4(), name)?;
    sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, ?)")
        .bind(group.access_group_uuid.to_string())
        .bind(&group.name)
        .execute(pool)
        .await?;
    Ok(group)
}

pub async fn rename_access_group(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
    name: &str,
) -> Result<AccessGroup, AccessGroupRepositoryError> {
    let group = AccessGroup::new(access_group_uuid, name)?;
    let result = sqlx::query("UPDATE access_groups SET name = ? WHERE access_group_uuid = ?")
        .bind(&group.name)
        .bind(access_group_uuid.to_string())
        .execute(pool)
        .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound.into());
    }
    Ok(group)
}

pub async fn delete_access_group(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
) -> Result<(), AccessGroupRepositoryError> {
    let result = sqlx::query("DELETE FROM access_groups WHERE access_group_uuid = ?")
        .bind(access_group_uuid.to_string())
        .execute(pool)
        .await?;
    if result.rows_affected() != 1 {
        return Err(sqlx::Error::RowNotFound.into());
    }
    Ok(())
}

pub async fn list_access_group_memberships(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
) -> Result<Vec<AccessGroupMembership>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, user_uuid
         FROM access_group_memberships
         WHERE access_group_uuid = ? ORDER BY user_uuid ASC",
    )
    .bind(access_group_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(group, user)| AccessGroupMembership {
            access_group_uuid: parse_uuid(group),
            user_uuid: parse_uuid(user),
        })
        .collect())
}

pub async fn replace_access_group_memberships(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
    user_uuids: &[Uuid],
) -> Result<(), AccessGroupRepositoryError> {
    reject_duplicates(user_uuids)?;
    let mut tx = pool.begin().await?;
    require_access_group(&mut tx, access_group_uuid).await?;
    for user_uuid in user_uuids {
        require_user(&mut tx, *user_uuid).await?;
    }
    sqlx::query("DELETE FROM access_group_memberships WHERE access_group_uuid = ?")
        .bind(access_group_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    for user_uuid in user_uuids {
        sqlx::query(
            "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
        )
        .bind(access_group_uuid.to_string())
        .bind(user_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn list_group_device_visibility_grants(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
) -> Result<Vec<DeviceVisibilityGrant>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, device_uuid
         FROM device_visibility_grants
         WHERE access_group_uuid = ? ORDER BY device_uuid ASC",
    )
    .bind(access_group_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(group, device)| DeviceVisibilityGrant {
            access_group_uuid: parse_uuid(group),
            device_uuid: parse_uuid(device),
        })
        .collect())
}

pub async fn replace_group_device_visibility_grants(
    pool: &SqlitePool,
    access_group_uuid: Uuid,
    device_uuids: &[Uuid],
) -> Result<(), AccessGroupRepositoryError> {
    reject_duplicates(device_uuids)?;
    let mut tx = pool.begin().await?;
    require_access_group(&mut tx, access_group_uuid).await?;
    for device_uuid in device_uuids {
        require_device(&mut tx, *device_uuid).await?;
    }
    sqlx::query("DELETE FROM device_visibility_grants WHERE access_group_uuid = ?")
        .bind(access_group_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    for device_uuid in device_uuids {
        sqlx::query(
            "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)",
        )
        .bind(access_group_uuid.to_string())
        .bind(device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
