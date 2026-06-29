use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth;
use crate::time_format::format_timestamp;
use time::OffsetDateTime;

pub struct UserRow {
    pub user_uuid: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<UserRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT user_uuid, username, password_hash, role FROM users ORDER BY username ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| UserRow {
            user_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
            username: row.1,
            password_hash: row.2,
            role: row.3,
        })
        .collect())
}

pub async fn count_users(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
    role: &str,
) -> Result<UserRow, anyhow::Error> {
    let user_uuid = Uuid::new_v4();
    let password_hash = auth::hash_password(password)?;
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO users (user_uuid, username, password_hash, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(user_uuid.to_string())
    .bind(username)
    .bind(password_hash)
    .bind(role)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(UserRow {
        user_uuid,
        username: username.to_string(),
        password_hash: String::new(),
        role: role.to_string(),
    })
}

pub async fn find_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT user_uuid, username, password_hash, role FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(user_uuid, username, password_hash, role)| UserRow {
        user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
        username,
        password_hash,
        role,
    }))
}

pub async fn find_user_by_uuid(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT user_uuid, username, password_hash, role FROM users WHERE user_uuid = ?",
    )
    .bind(user_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(user_uuid, username, password_hash, role)| UserRow {
        user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
        username,
        password_hash,
        role,
    }))
}