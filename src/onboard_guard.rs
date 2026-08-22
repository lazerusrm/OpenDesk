use std::collections::HashMap;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::login_throttle::LoginThrottle;

const REPLAY_TTL_SECS: i64 = 120;

#[derive(Default)]
pub struct OnboardGuard {
    throttle: LoginThrottle,
    replay: HashMap<(Uuid, i64), i64>,
}

impl OnboardGuard {
    pub fn is_blocked(&mut self, ip: &str, now: OffsetDateTime) -> bool {
        self.throttle.is_blocked("onboard", ip, now)
    }

    pub fn record_failure(&mut self, ip: &str, now: OffsetDateTime) {
        self.throttle.record_failure("onboard", ip, now);
    }

    pub fn record_success(&mut self, ip: &str) {
        self.throttle.record_success("onboard", ip);
    }

    /// Returns false when this authenticator timestep was already used.
    pub fn consume_timestep(
        &mut self,
        user_uuid: Uuid,
        timestep: i64,
        now: OffsetDateTime,
    ) -> bool {
        let now_unix = now.unix_timestamp();
        self.expire(now_unix);
        if self.replay.contains_key(&(user_uuid, timestep)) {
            return false;
        }
        self.replay.insert((user_uuid, timestep), now_unix);
        true
    }

    fn expire(&mut self, now_unix: i64) {
        self.replay
            .retain(|_, used_at| now_unix - *used_at < REPLAY_TTL_SECS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_rejects_the_same_timestep() {
        let mut guard = OnboardGuard::default();
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("ts");
        let user = Uuid::nil();
        assert!(guard.consume_timestep(user, 10, now));
        assert!(!guard.consume_timestep(user, 10, now));
        assert!(guard.consume_timestep(user, 11, now));
    }
}
