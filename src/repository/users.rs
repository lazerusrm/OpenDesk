use sqlx::SqlitePool;
use thiserror::Error;
use uuid::Uuid;

use crate::auth;
use crate::time_format::format_timestamp;
use time::OffsetDateTime;

pub struct UserRow {
    pub user_uuid: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub activation_state: String,
}

pub struct LegacyCredential {
    pub algorithm: String,
    pub verifier: String,
}

#[derive(Debug, Error)]
pub enum ResetUserPasswordError {
    #[error("user is missing or inactive")]
    MissingOrInactive,
    #[error("password hashing failed")]
    PasswordHash(#[from] auth::PasswordError),
    #[error("database operation failed")]
    Database(#[from] sqlx::Error),
}

pub async fn reset_user_password(
    pool: &SqlitePool,
    exact_username: &str,
    password: &str,
    now: OffsetDateTime,
) -> Result<(), ResetUserPasswordError> {
    let password_hash = auth::hash_password(password)?;
    let timestamp = format_timestamp(now);
    let audit_event_uuid = Uuid::new_v4();
    let mut tx = pool.begin().await?;
    let user_uuid: Option<String> = sqlx::query_scalar(
        "SELECT user_uuid FROM users WHERE username = ? AND activation_state = 'active'",
    )
    .bind(exact_username)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(user_uuid) = user_uuid else {
        tx.rollback().await?;
        return Err(ResetUserPasswordError::MissingOrInactive);
    };
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE user_uuid = ?")
        .bind(password_hash)
        .bind(&timestamp)
        .bind(&user_uuid)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE migration_legacy_credentials SET consumed_at = ? \
         WHERE user_uuid = ? AND consumed_at IS NULL",
    )
    .bind(&timestamp)
    .bind(&user_uuid)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM sessions WHERE user_uuid = ?")
        .bind(&user_uuid)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE client_access_tokens SET revoked_at = ? \
         WHERE user_uuid = ? AND revoked_at IS NULL",
    )
    .bind(&timestamp)
    .bind(&user_uuid)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO audit_events (
            audit_event_uuid, actor_user_uuid, action, object_type, object_uuid,
            outcome, source, detail_json, created_at
         ) VALUES (?, NULL, 'password_reset', 'user', ?, 'success', 'operator_cli', NULL, ?)",
    )
    .bind(audit_event_uuid.to_string())
    .bind(&user_uuid)
    .bind(&timestamp)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn list_users(pool: &SqlitePool) -> Result<Vec<UserRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT user_uuid, username, password_hash, role, activation_state FROM users ORDER BY username ASC",
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
            activation_state: row.4,
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
        activation_state: "active".to_string(),
    })
}

pub async fn activate_imported_user(
    pool: &SqlitePool,
    user_uuid: Uuid,
    password: &str,
) -> Result<bool, anyhow::Error> {
    let password_hash = auth::hash_password(password)?;
    let now = format_timestamp(OffsetDateTime::now_utc());
    let mut tx = pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE users
         SET password_hash = ?, activation_state = 'active', updated_at = ?
         WHERE user_uuid = ? AND activation_state = 'disabled'",
    )
    .bind(password_hash)
    .bind(&now)
    .bind(user_uuid.to_string())
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query("UPDATE migration_legacy_credentials SET consumed_at = ? WHERE user_uuid = ? AND consumed_at IS NULL")
        .bind(&now).bind(user_uuid.to_string()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn find_user_by_username(
    pool: &SqlitePool,
    username: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT user_uuid, username, password_hash, role, activation_state FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(user_uuid, username, password_hash, role, activation_state)| UserRow {
            user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
            username,
            password_hash,
            role,
            activation_state,
        },
    ))
}

pub async fn find_legacy_credential(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Option<LegacyCredential>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT verifier_algorithm, verifier
         FROM migration_legacy_credentials
         WHERE user_uuid = ? AND consumed_at IS NULL",
    )
    .bind(user_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(algorithm, verifier)| LegacyCredential {
        algorithm,
        verifier,
    }))
}

pub async fn upgrade_legacy_credential(
    pool: &SqlitePool,
    user_uuid: Uuid,
    algorithm: &str,
    verifier: &str,
    password: &str,
) -> Result<bool, anyhow::Error> {
    if auth::verify_legacy_bcrypt(password, algorithm, verifier).is_err() {
        return Ok(false);
    }
    let password_hash = auth::hash_password(password)?;
    let now = format_timestamp(OffsetDateTime::now_utc());
    let mut tx = pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE users
         SET password_hash = ?, updated_at = ?
         WHERE user_uuid = ? AND activation_state = 'active' AND password_hash = ''",
    )
    .bind(password_hash)
    .bind(&now)
    .bind(user_uuid.to_string())
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        tx.rollback().await?;
        return Ok(false);
    }
    let consumed = sqlx::query(
        "UPDATE migration_legacy_credentials
         SET consumed_at = ?
         WHERE user_uuid = ? AND verifier_algorithm = ? AND verifier = ?
           AND consumed_at IS NULL",
    )
    .bind(now)
    .bind(user_uuid.to_string())
    .bind(algorithm)
    .bind(verifier)
    .execute(&mut *tx)
    .await?;
    if consumed.rows_affected() != 1 {
        tx.rollback().await?;
        anyhow::bail!("credential upgrade rejected");
    }
    tx.commit().await?;
    Ok(true)
}

pub async fn authenticate_user_password(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> Result<Option<UserRow>, anyhow::Error> {
    let Some(user) = find_user_by_username(pool, username).await? else {
        return Ok(None);
    };
    if user.activation_state != "active" {
        return Ok(None);
    }
    if !user.password_hash.is_empty() {
        return Ok(auth::verify_password(password, &user.password_hash)
            .is_ok()
            .then_some(user));
    }
    let Some(credential) = find_legacy_credential(pool, user.user_uuid).await? else {
        return Ok(None);
    };
    if auth::verify_legacy_bcrypt(password, &credential.algorithm, &credential.verifier).is_err() {
        return Ok(None);
    }
    if upgrade_legacy_credential(
        pool,
        user.user_uuid,
        &credential.algorithm,
        &credential.verifier,
        password,
    )
    .await?
    {
        return Ok(Some(user));
    }
    let refreshed = find_user_by_username(pool, username).await?;
    Ok(refreshed.filter(|row| {
        row.activation_state == "active"
            && !row.password_hash.is_empty()
            && auth::verify_password(password, &row.password_hash).is_ok()
    }))
}

pub async fn find_user_by_uuid(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT user_uuid, username, password_hash, role, activation_state FROM users WHERE user_uuid = ?",
    )
    .bind(user_uuid.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(user_uuid, username, password_hash, role, activation_state)| UserRow {
            user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
            username,
            password_hash,
            role,
            activation_state,
        },
    ))
}
