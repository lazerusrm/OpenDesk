use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub const SESSION_DURATION_HOURS: i64 = 24;

pub fn new_session_uuid() -> Uuid {
    Uuid::new_v4()
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
        assert!(session_is_valid(expires, datetime!(2026-06-23 12:00:00 UTC)));
        assert!(!session_is_valid(expires, datetime!(2026-06-24 12:00:01 UTC)));
    }
}