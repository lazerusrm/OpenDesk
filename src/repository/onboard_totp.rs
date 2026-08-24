use sqlx::SqlitePool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::time_format::format_timestamp;

const REPLAY_TTL_SECS: i64 = 120;

pub struct OnboardTotpRecord {
    pub user_uuid: Uuid,
    pub username: String,
    pub secret: Vec<u8>,
}

pub async fn save_user_onboard_totp(
    pool: &SqlitePool,
    user_uuid: Uuid,
    secret: &[u8],
) -> Result<(), sqlx::Error> {
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO user_onboard_totp (user_uuid, secret_hex, created_at)
         VALUES (?, ?, ?)
         ON CONFLICT(user_uuid) DO UPDATE SET secret_hex = excluded.secret_hex, created_at = excluded.created_at",
    )
    .bind(user_uuid.to_string())
    .bind(hex::encode(secret))
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_user_onboard_totp(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_onboard_totp WHERE user_uuid = ?")
        .bind(user_uuid.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn user_has_onboard_totp(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<bool, sqlx::Error> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_onboard_totp WHERE user_uuid = ?")
            .bind(user_uuid.to_string())
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

pub async fn list_active_onboard_totp(
    pool: &SqlitePool,
    technician: Option<&str>,
) -> Result<Vec<OnboardTotpRecord>, sqlx::Error> {
    let technician = technician.map(str::trim).filter(|value| !value.is_empty());
    let rows = if let Some(username) = technician {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT t.user_uuid, u.username, t.secret_hex
             FROM user_onboard_totp t
             INNER JOIN users u ON u.user_uuid = t.user_uuid
             WHERE u.activation_state = 'active'
               AND (u.role = 'admin' OR u.role = 'operator')
               AND u.username = ?",
        )
        .bind(username)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT t.user_uuid, u.username, t.secret_hex
             FROM user_onboard_totp t
             INNER JOIN users u ON u.user_uuid = t.user_uuid
             WHERE u.activation_state = 'active'
               AND (u.role = 'admin' OR u.role = 'operator')
             ORDER BY u.username ASC",
        )
        .fetch_all(pool)
        .await?
    };
    Ok(rows
        .into_iter()
        .filter_map(|(user_uuid, username, secret_hex)| {
            let secret = hex::decode(secret_hex).ok()?;
            Some(OnboardTotpRecord {
                user_uuid: Uuid::parse_str(&user_uuid).ok()?,
                username,
                secret,
            })
        })
        .collect())
}

pub async fn consume_onboard_totp_timestep(
    pool: &SqlitePool,
    user_uuid: Uuid,
    timestep: i64,
    now: OffsetDateTime,
) -> Result<bool, sqlx::Error> {
    let cutoff = format_timestamp(now - Duration::seconds(REPLAY_TTL_SECS));
    sqlx::query("DELETE FROM onboard_totp_replay WHERE used_at < ?")
        .bind(&cutoff)
        .execute(pool)
        .await?;
    let used_at = format_timestamp(now);
    let result = sqlx::query(
        "INSERT INTO onboard_totp_replay (user_uuid, timestep, used_at) VALUES (?, ?, ?)",
    )
    .bind(user_uuid.to_string())
    .bind(timestep)
    .bind(&used_at)
    .execute(pool)
    .await;
    match result {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => Ok(false),
        Err(error) => Err(error),
    }
}
