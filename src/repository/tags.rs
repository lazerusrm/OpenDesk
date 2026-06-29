use std::collections::HashMap;

use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::tag::{normalize_tag_name, Tag, TagDraft};
use crate::time_format::format_timestamp;

pub async fn list_tags(pool: &SqlitePool) -> Result<Vec<Tag>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT tag_uuid, name FROM tags ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Tag {
            tag_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
            name: row.1,
        })
        .collect())
}

pub async fn find_tag_by_uuid(
    pool: &SqlitePool,
    tag_uuid: Uuid,
) -> Result<Option<Tag>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT tag_uuid, name FROM tags WHERE tag_uuid = ?",
    )
    .bind(tag_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| Tag {
        tag_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
        name: row.1,
    }))
}

pub async fn create_tag(pool: &SqlitePool, draft: &TagDraft) -> Result<Tag, sqlx::Error> {
    let tag_uuid = Uuid::new_v4();
    let name = normalize_tag_name(&draft.name);
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO tags (tag_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(tag_uuid.to_string())
    .bind(&name)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    find_tag_by_uuid(pool, tag_uuid)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn list_tag_uuids_for_device(
    pool: &SqlitePool,
    device_uuid: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT tag_uuid FROM device_tags WHERE device_uuid = ? ORDER BY tag_uuid ASC",
    )
    .bind(device_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Uuid::parse_str(&row.0).expect("stored uuid"))
        .collect())
}

pub async fn list_tags_for_device(
    pool: &SqlitePool,
    device_uuid: Uuid,
) -> Result<Vec<Tag>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT t.tag_uuid, t.name
         FROM tags t
         INNER JOIN device_tags dt ON dt.tag_uuid = t.tag_uuid
         WHERE dt.device_uuid = ?
         ORDER BY t.name ASC",
    )
    .bind(device_uuid.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Tag {
            tag_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
            name: row.1,
        })
        .collect())
}

pub async fn list_device_tag_links(
    pool: &SqlitePool,
) -> Result<Vec<(Uuid, Uuid)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT device_uuid, tag_uuid FROM device_tags ORDER BY device_uuid ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                Uuid::parse_str(&row.0).expect("stored uuid"),
                Uuid::parse_str(&row.1).expect("stored uuid"),
            )
        })
        .collect())
}

pub async fn list_device_tag_names_map(
    pool: &SqlitePool,
) -> Result<HashMap<Uuid, Vec<String>>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT dt.device_uuid, t.name
         FROM device_tags dt
         INNER JOIN tags t ON t.tag_uuid = dt.tag_uuid
         ORDER BY t.name ASC",
    )
    .fetch_all(pool)
    .await?;
    let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (device_uuid, name) in rows {
        let device_uuid = Uuid::parse_str(&device_uuid).expect("stored uuid");
        map.entry(device_uuid).or_default().push(name);
    }
    Ok(map)
}

pub async fn set_device_tags(
    pool: &SqlitePool,
    device_uuid: Uuid,
    tag_uuids: &[Uuid],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM device_tags WHERE device_uuid = ?")
        .bind(device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    for tag_uuid in tag_uuids {
        sqlx::query("INSERT INTO device_tags (device_uuid, tag_uuid) VALUES (?, ?)")
            .bind(device_uuid.to_string())
            .bind(tag_uuid.to_string())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}