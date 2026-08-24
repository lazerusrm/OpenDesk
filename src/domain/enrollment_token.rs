use rand::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub const ENROLLMENT_TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentTokenRecord {
    pub enrollment_token_uuid: Uuid,
    pub token_hash: String,
    pub label: String,
    pub site_uuid: Option<Uuid>,
    pub expires_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
    pub created_by_user_uuid: Option<Uuid>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnrollmentTokenError {
    #[error("enrollment token label must not be empty")]
    EmptyLabel,
    #[error("enrollment token has expired")]
    Expired,
    #[error("enrollment token has been revoked")]
    Revoked,
    #[error("enrollment token is invalid")]
    Invalid,
    #[error("enrollment token expiry must be 7, 30, 90, or 365 days, or empty")]
    InvalidExpiry,
}

pub fn generate_enrollment_token_value() -> String {
    let mut bytes = [0u8; ENROLLMENT_TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Accept only the canonical 64-character lowercase hex token in public paths.
pub fn is_onboard_token_value(value: &str) -> bool {
    value.len() == ENROLLMENT_TOKEN_BYTES * 2
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn onboard_url(public_base_url: &str, token_value: &str) -> String {
    format!(
        "{}/onboard/{}",
        public_base_url.trim_end_matches('/'),
        token_value
    )
}

pub fn hash_enrollment_token_value(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)
}

pub fn validate_enrollment_token_label(label: &str) -> Result<(), EnrollmentTokenError> {
    if label.trim().is_empty() {
        return Err(EnrollmentTokenError::EmptyLabel);
    }
    Ok(())
}

/// Parse the dashboard expiry allowlist. Empty means no expiry; unknown values fail closed.
pub fn enrollment_token_expires_at_from_days(
    value: &str,
    now: OffsetDateTime,
) -> Result<Option<OffsetDateTime>, EnrollmentTokenError> {
    match value.trim() {
        "" => Ok(None),
        "7" => Ok(Some(now + Duration::days(7))),
        "30" => Ok(Some(now + Duration::days(30))),
        "90" => Ok(Some(now + Duration::days(90))),
        "365" => Ok(Some(now + Duration::days(365))),
        _ => Err(EnrollmentTokenError::InvalidExpiry),
    }
}

pub fn enrollment_token_is_active(
    record: &EnrollmentTokenRecord,
    now: OffsetDateTime,
) -> Result<(), EnrollmentTokenError> {
    if record.revoked_at.is_some() {
        return Err(EnrollmentTokenError::Revoked);
    }
    if let Some(expires_at) = record.expires_at {
        if now >= expires_at {
            return Err(EnrollmentTokenError::Expired);
        }
    }
    Ok(())
}

pub fn verify_enrollment_token_value(
    record: &EnrollmentTokenRecord,
    provided_value: &str,
    now: OffsetDateTime,
) -> Result<(), EnrollmentTokenError> {
    enrollment_token_is_active(record, now)?;
    let provided_hash = hash_enrollment_token_value(provided_value);
    if provided_hash != record.token_hash {
        return Err(EnrollmentTokenError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn active_record() -> EnrollmentTokenRecord {
        EnrollmentTokenRecord {
            enrollment_token_uuid: Uuid::new_v4(),
            token_hash: hash_enrollment_token_value("test-token-value"),
            label: "lab".to_string(),
            site_uuid: None,
            expires_at: Some(datetime!(2026-12-31 00:00:00 UTC)),
            revoked_at: None,
            created_by_user_uuid: None,
        }
    }

    #[test]
    fn verify_enrollment_token_value_accepts_matching_token() {
        let record = active_record();
        assert!(verify_enrollment_token_value(
            &record,
            "test-token-value",
            datetime!(2026-06-23 00:00:00 UTC)
        )
        .is_ok());
    }

    #[test]
    fn verify_enrollment_token_value_rejects_revoked_token() {
        let mut record = active_record();
        record.revoked_at = Some(datetime!(2026-06-01 00:00:00 UTC));
        assert_eq!(
            verify_enrollment_token_value(
                &record,
                "test-token-value",
                datetime!(2026-06-23 00:00:00 UTC)
            ),
            Err(EnrollmentTokenError::Revoked)
        );
    }

    #[test]
    fn expiry_days_allowlist_is_exact() {
        let now = datetime!(2026-08-21 00:00:00 UTC);
        assert_eq!(enrollment_token_expires_at_from_days("", now), Ok(None));
        assert_eq!(
            enrollment_token_expires_at_from_days("7", now),
            Ok(Some(datetime!(2026-08-28 00:00:00 UTC)))
        );
        assert_eq!(
            enrollment_token_expires_at_from_days("365", now),
            Ok(Some(datetime!(2027-08-21 00:00:00 UTC)))
        );
        assert_eq!(
            enrollment_token_expires_at_from_days("14", now),
            Err(EnrollmentTokenError::InvalidExpiry)
        );
        assert_eq!(
            enrollment_token_expires_at_from_days("never", now),
            Err(EnrollmentTokenError::InvalidExpiry)
        );
    }

    #[test]
    fn onboard_token_value_accepts_canonical_hex_only() {
        let value = generate_enrollment_token_value();
        assert!(is_onboard_token_value(&value));
        assert!(!is_onboard_token_value(&value.to_uppercase()));
        assert!(!is_onboard_token_value("abc"));
        assert!(!is_onboard_token_value("../etc/passwd"));
        assert_eq!(
            onboard_url("https://rd.example.com/", &value),
            format!("https://rd.example.com/onboard/{value}")
        );
    }
}
