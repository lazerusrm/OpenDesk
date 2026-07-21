use rand::{rngs::OsRng, RngCore};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub const SESSION_DURATION_HOURS: i64 = 24;

pub fn new_session_uuid() -> Uuid {
    Uuid::new_v4()
}

pub fn new_csrf_token() -> String {
    let mut token = [0_u8; 32];
    OsRng.fill_bytes(&mut token);
    hex::encode(token)
}

pub fn session_expires_at(now: OffsetDateTime) -> OffsetDateTime {
    now + Duration::hours(SESSION_DURATION_HOURS)
}

pub fn session_is_valid(expires_at: OffsetDateTime, now: OffsetDateTime) -> bool {
    now < expires_at
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn session_expires_24_hours_after_creation() {
        let now = datetime!(2026-06-23 12:00:00 UTC);
        let expires = session_expires_at(now);
        assert_eq!(expires, datetime!(2026-06-24 12:00:00 UTC));
    }

    #[test]
    fn session_validity_respects_expiry() {
        let expires = datetime!(2026-06-24 12:00:00 UTC);
        assert!(session_is_valid(
            expires,
            datetime!(2026-06-23 12:00:00 UTC)
        ));
        assert!(!session_is_valid(
            expires,
            datetime!(2026-06-24 12:00:01 UTC)
        ));
    }

    #[test]
    fn csrf_tokens_are_random_and_32_bytes() {
        let first = new_csrf_token();
        let second = new_csrf_token();
        assert_eq!(first.len(), 64);
        assert_ne!(first, second);
    }
}
