use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::enrollment_token::{
    generate_enrollment_token_value, hash_enrollment_token_value, EnrollmentTokenRecord,
};
use crate::time_format::{format_timestamp, parse_timestamp};

pub struct CreatedEnrollmentToken {
    pub record: EnrollmentTokenRecord,
    pub token_value: String,
}

pub async fn create_enrollment_token(
    pool: &SqlitePool,
    label: &str,
    site_uuid: Option<Uuid>,
    expires_at: Option<OffsetDateTime>,
    created_by_user_uuid: Option<Uuid>,
) -> Result<CreatedEnrollmentToken, sqlx::Error> {
    let enrollment_token_uuid = Uuid::new_v4();
    let token_value = generate_enrollment_token_value();
    let token_hash = hash_enrollment_token_value(&token_value);
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO enrollment_tokens (
            enrollment_token_uuid, token_hash, label, site_uuid, expires_at, created_at, created_by_user_uuid
         ) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(enrollment_token_uuid.to_string())
    .bind(&token_hash)
    .bind(label)
    .bind(site_uuid.map(|value| value.to_string()))
    .bind(expires_at.map(format_timestamp))
    .bind(&now)
    .bind(created_by_user_uuid.map(|value| value.to_string()))
    .execute(pool)
    .await?;
    Ok(CreatedEnrollmentToken {
        record: EnrollmentTokenRecord {
            enrollment_token_uuid,
            token_hash,
            label: label.to_string(),
            site_uuid,
            expires_at,
            revoked_at: None,
        },
        token_value,
    })
}

pub async fn list_enrollment_tokens(
    pool: &SqlitePool,
) -> Result<Vec<EnrollmentTokenRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )>(
        "SELECT enrollment_token_uuid, token_hash, label, site_uuid, expires_at, revoked_at
         FROM enrollment_tokens ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| EnrollmentTokenRecord {
            enrollment_token_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
            token_hash: row.1,
            label: row.2,
            site_uuid: row.3.and_then(|value| Uuid::parse_str(&value).ok()),
            expires_at: row.4.and_then(|value| parse_timestamp(&value)),
            revoked_at: row.5.and_then(|value| parse_timestamp(&value)),
        })
        .collect())
}

pub async fn find_enrollment_token_by_hash(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<EnrollmentTokenRecord>, sqlx::Error> {
    let row = sqlx::query_as::<_, (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )>(
        "SELECT enrollment_token_uuid, token_hash, label, site_uuid, expires_at, revoked_at
         FROM enrollment_tokens WHERE token_hash = ?",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| EnrollmentTokenRecord {
        enrollment_token_uuid: Uuid::parse_str(&row.0).expect("stored uuid"),
        token_hash: row.1,
        label: row.2,
        site_uuid: row.3.and_then(|value| Uuid::parse_str(&value).ok()),
        expires_at: row.4.and_then(|value| parse_timestamp(&value)),
        revoked_at: row.5.and_then(|value| parse_timestamp(&value)),
    }))
}

pub async fn revoke_enrollment_token(
    pool: &SqlitePool,
    enrollment_token_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query("UPDATE enrollment_tokens SET revoked_at = ? WHERE enrollment_token_uuid = ?")
        .bind(&now)
        .bind(enrollment_token_uuid.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn record_endpoint_checkin(
    pool: &SqlitePool,
    device_uuid: Uuid,
    enrollment_token_uuid: Uuid,
    rustdesk_id: Option<&str>,
    hostname: Option<&str>,
    os_family: Option<&str>,
    os_version: Option<&str>,
    architecture: Option<&str>,
    rustdesk_version: Option<&str>,
) -> Result<(), sqlx::Error> {
    let endpoint_checkin_uuid = Uuid::new_v4();
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO endpoint_checkins (
            endpoint_checkin_uuid, device_uuid, enrollment_token_uuid, rustdesk_id, hostname,
            os_family, os_version, architecture, rustdesk_version, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(endpoint_checkin_uuid.to_string())
    .bind(device_uuid.to_string())
    .bind(enrollment_token_uuid.to_string())
    .bind(rustdesk_id)
    .bind(hostname)
    .bind(os_family)
    .bind(os_version)
    .bind(architecture)
    .bind(rustdesk_version)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}