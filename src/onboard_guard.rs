use time::OffsetDateTime;

use crate::login_throttle::LoginThrottle;

#[derive(Default)]
pub struct OnboardGuard {
    throttle: LoginThrottle,
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
}
