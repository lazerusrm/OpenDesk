use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use thiserror::Error;
use time::{Duration, OffsetDateTime};

pub const CLIENT_ACCESS_TOKEN_BYTES: usize = 32;
pub const CLIENT_ACCESS_TOKEN_LIFETIME: Duration = Duration::days(7);

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClientAccessTokenError {
    #[error("client access token key must contain at least 32 bytes")]
    InvalidKey,
    #[error("client identity is invalid")]
    InvalidClientIdentity,
}

pub fn validate_client_identity(
    rustdesk_id: &str,
    client_uuid: &str,
) -> Result<(), ClientAccessTokenError> {
    if !valid_identity_value(rustdesk_id, 64) || !valid_identity_value(client_uuid, 128) {
        return Err(ClientAccessTokenError::InvalidClientIdentity);
    }
    Ok(())
}

pub fn generate_client_access_token() -> String {
    let mut bytes = [0u8; CLIENT_ACCESS_TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn digest_client_access_token(
    key: &[u8],
    value: &str,
) -> Result<String, ClientAccessTokenError> {
    if key.len() < 32 {
        return Err(ClientAccessTokenError::InvalidKey);
    }
    let mut mac =
        HmacSha256::new_from_slice(key).map_err(|_| ClientAccessTokenError::InvalidKey)?;
    mac.update(value.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

pub fn client_access_token_expires_at(now: OffsetDateTime) -> OffsetDateTime {
    now + CLIENT_ACCESS_TOKEN_LIFETIME
}

fn valid_identity_value(value: &str, maximum_length: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_length
        && !value.chars().any(|character| character.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn digest_is_keyed_and_key_requires_adequate_entropy() {
        let first = digest_client_access_token(&[1; 32], "token").expect("digest");
        let second = digest_client_access_token(&[2; 32], "token").expect("digest");
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);
        assert_eq!(
            digest_client_access_token(&[1; 31], "token"),
            Err(ClientAccessTokenError::InvalidKey)
        );
    }

    #[test]
    fn identity_and_expiry_are_bounded() {
        assert!(validate_client_identity("123456789", "client-uuid").is_ok());
        assert!(validate_client_identity("", "client-uuid").is_err());
        assert!(validate_client_identity("123", "bad\nvalue").is_err());
        let now = datetime!(2026-07-22 00:00:00 UTC);
        assert_eq!(client_access_token_expires_at(now), now + Duration::days(7));
    }
}
