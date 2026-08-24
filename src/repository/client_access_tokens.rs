use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::client_access_token::{
    client_access_token_expires_at, digest_client_access_token, generate_client_access_token,
};
use crate::time_format::format_timestamp;

#[derive(Debug)]
pub struct AuthenticatedClientToken {
    pub client_access_token_uuid: Uuid,
    pub user_uuid: Uuid,
}

pub async fn issue_client_access_token(
    pool: &SqlitePool,
    key: &[u8],
    user_uuid: Uuid,
    rustdesk_id: &str,
    client_uuid: &str,
    now: OffsetDateTime,
) -> Result<String, anyhow::Error> {
    let value = generate_client_access_token();
    let digest = digest_client_access_token(key, &value)?;
    let token_uuid = Uuid::new_v4();
    let now_text = format_timestamp(now);
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "UPDATE client_access_tokens SET revoked_at = ?
         WHERE user_uuid = ? AND rustdesk_id = ? AND client_uuid = ? AND revoked_at IS NULL",
    )
    .bind(&now_text)
    .bind(user_uuid.to_string())
    .bind(rustdesk_id)
    .bind(client_uuid)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO client_access_tokens (
            client_access_token_uuid, token_digest, user_uuid, rustdesk_id, client_uuid,
            expires_at, created_at, last_used_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(token_uuid.to_string())
    .bind(digest)
    .bind(user_uuid.to_string())
    .bind(rustdesk_id)
    .bind(client_uuid)
    .bind(format_timestamp(client_access_token_expires_at(now)))
    .bind(&now_text)
    .bind(&now_text)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(value)
}

pub async fn authenticate_client_access_token(
    pool: &SqlitePool,
    key: &[u8],
    value: &str,
    rustdesk_id: &str,
    client_uuid: &str,
    now: OffsetDateTime,
) -> Result<Option<AuthenticatedClientToken>, anyhow::Error> {
    let digest = digest_client_access_token(key, value)?;
    let row = sqlx::query_as::<_, (String, String)>(
        "UPDATE client_access_tokens AS token
         SET last_used_at = ?
         WHERE token_digest = ? AND rustdesk_id = ? AND client_uuid = ?
           AND revoked_at IS NULL AND unixepoch(expires_at) > unixepoch(?)
           AND EXISTS (
               SELECT 1 FROM users
               WHERE users.user_uuid = token.user_uuid
                 AND users.activation_state = 'active'
           )
         RETURNING client_access_token_uuid, user_uuid",
    )
    .bind(format_timestamp(now))
    .bind(digest)
    .bind(rustdesk_id)
    .bind(client_uuid)
    .bind(format_timestamp(now))
    .fetch_optional(pool)
    .await?;
    let Some((token_uuid, user_uuid)) = row else {
        return Ok(None);
    };
    Ok(Some(AuthenticatedClientToken {
        client_access_token_uuid: Uuid::parse_str(&token_uuid)?,
        user_uuid: Uuid::parse_str(&user_uuid)?,
    }))
}

pub async fn authenticate_client_bearer_token(
    pool: &SqlitePool,
    key: &[u8],
    value: &str,
    now: OffsetDateTime,
) -> Result<Option<AuthenticatedClientToken>, anyhow::Error> {
    let digest = digest_client_access_token(key, value)?;
    let row = sqlx::query_as::<_, (String, String)>(
        "UPDATE client_access_tokens AS token
         SET last_used_at = ?
         WHERE token_digest = ?
           AND revoked_at IS NULL AND unixepoch(expires_at) > unixepoch(?)
           AND EXISTS (
               SELECT 1 FROM users
               WHERE users.user_uuid = token.user_uuid
                 AND users.activation_state = 'active'
           )
         RETURNING client_access_token_uuid, user_uuid",
    )
    .bind(format_timestamp(now))
    .bind(digest)
    .bind(format_timestamp(now))
    .fetch_optional(pool)
    .await?;
    let Some((token_uuid, user_uuid)) = row else {
        return Ok(None);
    };
    Ok(Some(AuthenticatedClientToken {
        client_access_token_uuid: Uuid::parse_str(&token_uuid)?,
        user_uuid: Uuid::parse_str(&user_uuid)?,
    }))
}

pub async fn authorize_client_token_for_device(
    pool: &SqlitePool,
    key: &[u8],
    value: &str,
    rustdesk_id: &str,
    now: OffsetDateTime,
) -> Result<bool, anyhow::Error> {
    let digest = digest_client_access_token(key, value)?;
    let authorized: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM client_access_tokens token
            JOIN users user ON user.user_uuid = token.user_uuid
            JOIN devices device ON device.rustdesk_id = ? AND device.archived = 0
            WHERE token.token_digest = ?
              AND token.revoked_at IS NULL
              AND unixepoch(token.expires_at) > unixepoch(?)
              AND user.activation_state = 'active'
              AND (SELECT COUNT(*) FROM devices candidate
                   WHERE candidate.rustdesk_id = ? AND candidate.archived = 0) = 1
              AND (
                  EXISTS (
                      SELECT 1 FROM user_device_visibility_grants direct_grant
                      WHERE direct_grant.user_uuid = token.user_uuid
                        AND direct_grant.device_uuid = device.device_uuid
                  ) OR EXISTS (
                      SELECT 1 FROM access_group_memberships membership
                      JOIN device_visibility_grants group_grant
                        ON group_grant.access_group_uuid = membership.access_group_uuid
                      WHERE membership.user_uuid = token.user_uuid
                        AND group_grant.device_uuid = device.device_uuid
                  )
              )
        )",
    )
    .bind(rustdesk_id)
    .bind(digest)
    .bind(format_timestamp(now))
    .bind(rustdesk_id)
    .fetch_one(pool)
    .await?;
    Ok(authorized)
}

pub async fn revoke_client_access_token(
    pool: &SqlitePool,
    token_uuid: Uuid,
    now: OffsetDateTime,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE client_access_tokens SET revoked_at = ?
         WHERE client_access_token_uuid = ? AND revoked_at IS NULL",
    )
    .bind(format_timestamp(now))
    .bind(token_uuid.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}
