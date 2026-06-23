use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::session::{new_session_uuid, session_expires_at};
use crate::time_format::{format_timestamp, parse_timestamp};

pub struct SessionRow {
    pub session_uuid: Uuid,
    pub user_uuid: Uuid,
    pub expires_at: OffsetDateTime,
}

pub async fn create_session(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<SessionRow, sqlx::Error> {
    let session_uuid = new_session_uuid();
    let now = OffsetDateTime::now_utc();
    let expires_at = session_expires_at(now);
    sqlx::query(
        "INSERT INTO sessions (session_uuid, user_uuid, expires_at, created_at)
         VALUES (?, ?, ?, ?)",
    )
    .bind(session_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(format_timestamp(expires_at))
    .bind(format_timestamp(now))
    .execute(pool)
    .await?;
    Ok(SessionRow {
        session_uuid,
        user_uuid,
        expires_at,
    })
}

pub async fn find_session(
    pool: &SqlitePool,
    session_uuid: Uuid,
) -> Result<Option<SessionRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT session_uuid, user_uuid, expires_at FROM sessions WHERE session_uuid = ?",
    )
    .bind(session_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(session_uuid, user_uuid, expires_at)| SessionRow {
        session_uuid: Uuid::parse_str(&session_uuid).expect("stored uuid"),
        user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
        expires_at: parse_timestamp(&expires_at).expect("stored timestamp"),
    }))
}

pub async fn delete_session(pool: &SqlitePool, session_uuid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE session_uuid = ?")
        .bind(session_uuid.to_string())
        .execute(pool)
        .await?;
    Ok(())
}