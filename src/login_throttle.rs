use std::collections::HashMap;

use time::OffsetDateTime;

const WINDOW_SECS: i64 = 15 * 60;
const LOCK_SECS: i64 = 15 * 60;
const MAX_USER_IP_FAILURES: u32 = 5;
const MAX_IP_FAILURES: u32 = 20;

#[derive(Default)]
pub struct LoginThrottle {
    user_ip: HashMap<(String, String), Window>,
    ip: HashMap<String, Window>,
}

#[derive(Clone, Copy)]
struct Window {
    started_at: i64,
    failures: u32,
    locked_until: i64,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            started_at: 0,
            failures: 0,
            locked_until: 0,
        }
    }
}

impl LoginThrottle {
    pub fn is_blocked(&mut self, username: &str, ip: &str, now: OffsetDateTime) -> bool {
        let now_unix = now.unix_timestamp();
        self.expire(now_unix);
        window_blocked(
            self.user_ip.get(&(username.to_string(), ip.to_string())),
            now_unix,
        ) || window_blocked(self.ip.get(ip), now_unix)
    }

    pub fn record_failure(&mut self, username: &str, ip: &str, now: OffsetDateTime) {
        let now_unix = now.unix_timestamp();
        bump(
            self.user_ip
                .entry((username.to_string(), ip.to_string()))
                .or_default(),
            now_unix,
            MAX_USER_IP_FAILURES,
        );
        bump(
            self.ip.entry(ip.to_string()).or_default(),
            now_unix,
            MAX_IP_FAILURES,
        );
    }

    pub fn record_success(&mut self, username: &str, ip: &str) {
        self.user_ip.remove(&(username.to_string(), ip.to_string()));
        self.ip.remove(ip);
    }

    fn expire(&mut self, now_unix: i64) {
        self.user_ip.retain(|_, window| {
            window.locked_until > now_unix || now_unix - window.started_at < WINDOW_SECS
        });
        self.ip.retain(|_, window| {
            window.locked_until > now_unix || now_unix - window.started_at < WINDOW_SECS
        });
    }
}

fn window_blocked(window: Option<&Window>, now_unix: i64) -> bool {
    window.is_some_and(|window| window.locked_until > now_unix)
}

fn bump(window: &mut Window, now_unix: i64, max_failures: u32) {
    if window.locked_until > now_unix {
        return;
    }
    if window.started_at == 0 || now_unix - window.started_at >= WINDOW_SECS {
        *window = Window {
            started_at: now_unix,
            failures: 1,
            locked_until: 0,
        };
        return;
    }
    window.failures = window.failures.saturating_add(1);
    if window.failures >= max_failures {
        window.locked_until = now_unix + LOCK_SECS;
    }
}

pub fn request_ip(headers: &axum::http::HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn five_failures_lock_the_username_and_ip() {
        let mut throttle = LoginThrottle::default();
        let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("ts");
        for _ in 0..5 {
            assert!(!throttle.is_blocked("admin", "203.0.113.10", now));
            throttle.record_failure("admin", "203.0.113.10", now);
        }
        assert!(throttle.is_blocked("admin", "203.0.113.10", now));
        assert!(!throttle.is_blocked("admin", "203.0.113.11", now));
        throttle.record_success("admin", "203.0.113.10");
        assert!(!throttle.is_blocked("admin", "203.0.113.10", now));
    }
}
