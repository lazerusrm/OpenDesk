use rand::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::OffsetDateTime;
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
}

pub fn generate_enrollment_token_value() -> String {
    let mut bytes = [0u8; ENROLLMENT_TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
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
}