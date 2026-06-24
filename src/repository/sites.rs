use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::site::{normalize_site_name, Site, SiteDraft};
use crate::time_format::format_timestamp;

pub async fn list_sites(pool: &SqlitePool) -> Result<Vec<Site>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT site_uuid, name FROM sites ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Site {
            site_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
            name: row.1,
        })
        .collect())
}

pub async fn find_site_by_uuid(
    pool: &SqlitePool,
    site_uuid: Uuid,
) -> Result<Option<Site>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT site_uuid, name FROM sites WHERE site_uuid = ?",
    )
    .bind(site_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| Site {
        site_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
        name: row.1,
    }))
}

pub async fn create_site(pool: &SqlitePool, draft: &SiteDraft) -> Result<Site, sqlx::Error> {
    let site_uuid = Uuid::new_v4();
    let name = normalize_site_name(&draft.name);
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO sites (site_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
    )
    .bind(site_uuid.to_string())
    .bind(&name)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    find_site_by_uuid(pool, site_uuid)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}