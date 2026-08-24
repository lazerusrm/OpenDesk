use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::domain::server_config::ServerConfig;

pub const HBBS_TCP_PORT: u16 = 21116;
pub const HBBR_TCP_PORT: u16 = 21117;
pub const HEALTH_PROBE_TIMEOUT_MS: u64 = 1500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheckResult {
    pub label: String,
    pub target: String,
    pub status: String,
    pub detail: String,
}

pub fn parse_server_host_port(value: &str) -> (String, Option<u16>) {
    let value = value.trim();
    if value.is_empty() {
        return (String::new(), None);
    }
    if let Some(inner) = value.strip_prefix('[') {
        if let Some(end) = inner.find(']') {
            let host = inner[..end].trim().to_string();
            let port = inner[end + 1..].strip_prefix(':').and_then(parse_tcp_port);
            return (host, port);
        }
        return (value.to_string(), None);
    }
    if let Some((host, suffix)) = value.rsplit_once(':') {
        if !host.contains(':') {
            if let Some(port) = parse_tcp_port(suffix) {
                return (host.to_string(), Some(port));
            }
        }
    }
    (value.to_string(), None)
}

fn parse_tcp_port(suffix: &str) -> Option<u16> {
    let suffix = suffix.trim();
    if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

pub fn host_from_server_value(value: &str) -> String {
    parse_server_host_port(value).0
}

pub fn tcp_probe_port(value: &str, default: u16) -> u16 {
    parse_server_host_port(value).1.unwrap_or(default)
}

pub fn public_key_fingerprint(public_key: &str) -> String {
    let trimmed = public_key.trim();
    if trimmed.is_empty() {
        return "-".to_string();
    }
    let digest = Sha256::digest(trimmed.as_bytes());
    format!("sha256:{}", hex::encode(digest))
}

pub fn dns_resolve_check(hostname: &str) -> HealthCheckResult {
    let host = host_from_server_value(hostname);
    let target = format!("dns:{host}");
    if host.is_empty() {
        return HealthCheckResult {
            label: "DNS".to_string(),
            target,
            status: "skipped".to_string(),
            detail: "hostname not configured".to_string(),
        };
    }
    match (host.as_str(), 0u16).to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => HealthCheckResult {
                label: "DNS".to_string(),
                target,
                status: "ok".to_string(),
                detail: format!("resolved to {addr}"),
            },
            None => HealthCheckResult {
                label: "DNS".to_string(),
                target,
                status: "failed".to_string(),
                detail: "no addresses returned".to_string(),
            },
        },
        Err(error) => HealthCheckResult {
            label: "DNS".to_string(),
            target,
            status: "failed".to_string(),
            detail: error.to_string(),
        },
    }
}

pub fn tcp_port_check(host: &str, port: u16, timeout_ms: u64) -> HealthCheckResult {
    let hostname = host_from_server_value(host);
    let target = format!("tcp:{hostname}:{port}");
    if hostname.is_empty() {
        return HealthCheckResult {
            label: format!("TCP {port}"),
            target,
            status: "skipped".to_string(),
            detail: "host not configured".to_string(),
        };
    }
    let socket_addr = match (hostname.as_str(), port).to_socket_addrs() {
        Ok(mut addrs) => match addrs.find(|addr| addr.is_ipv4() || addr.is_ipv6()) {
            Some(addr) => addr,
            None => {
                return HealthCheckResult {
                    label: format!("TCP {port}"),
                    target,
                    status: "failed".to_string(),
                    detail: "no socket addresses returned".to_string(),
                };
            }
        },
        Err(error) => {
            return HealthCheckResult {
                label: format!("TCP {port}"),
                target,
                status: "failed".to_string(),
                detail: error.to_string(),
            };
        }
    };
    match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout_ms)) {
        Ok(_) => HealthCheckResult {
            label: format!("TCP {port}"),
            target,
            status: "ok".to_string(),
            detail: "connection accepted".to_string(),
        },
        Err(error) => HealthCheckResult {
            label: format!("TCP {port}"),
            target,
            status: "failed".to_string(),
            detail: error.to_string(),
        },
    }
}

pub fn build_health_checks(config: &ServerConfig) -> Vec<HealthCheckResult> {
    let timeout = HEALTH_PROBE_TIMEOUT_MS;
    vec![
        dns_resolve_check(&config.id_server),
        dns_resolve_check(&config.relay_server),
        tcp_port_check(
            &config.id_server,
            tcp_probe_port(&config.id_server, HBBS_TCP_PORT),
            timeout,
        ),
        tcp_port_check(
            &config.relay_server,
            tcp_probe_port(&config.relay_server, HBBR_TCP_PORT),
            timeout,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server_config::default_server_config;

    #[test]
    fn host_from_server_value_strips_port_suffix() {
        assert_eq!(
            host_from_server_value("rd.example.com:21116"),
            "rd.example.com"
        );
        assert_eq!(host_from_server_value("127.0.0.1:21118"), "127.0.0.1");
        assert_eq!(host_from_server_value("rd.example.com"), "rd.example.com");
    }

    #[test]
    fn parse_server_host_port_keeps_unbracketed_ipv6() {
        assert_eq!(
            parse_server_host_port("2001:db8::1"),
            ("2001:db8::1".to_string(), None)
        );
        assert_eq!(
            parse_server_host_port("[2001:db8::1]"),
            ("2001:db8::1".to_string(), None)
        );
        assert_eq!(
            parse_server_host_port("[2001:db8::1]:21116"),
            ("2001:db8::1".to_string(), Some(21116))
        );
        assert_eq!(host_from_server_value("[2001:db8::1]:21116"), "2001:db8::1");
        assert_eq!(host_from_server_value("2001:db8::1"), "2001:db8::1");
    }

    #[test]
    fn tcp_probe_port_honors_explicit_suffix_else_default() {
        assert_eq!(tcp_probe_port("rd.example.com:12345", HBBS_TCP_PORT), 12345);
        assert_eq!(
            tcp_probe_port("rd.example.com", HBBS_TCP_PORT),
            HBBS_TCP_PORT
        );
        assert_eq!(tcp_probe_port("[2001:db8::1]:21118", HBBS_TCP_PORT), 21118);
        assert_eq!(tcp_probe_port("2001:db8::1", HBBR_TCP_PORT), HBBR_TCP_PORT);
        assert_eq!(tcp_probe_port("rd.example.com:notaport", 21116), 21116);
    }

    #[test]
    fn public_key_fingerprint_is_stable_sha256_prefix() {
        let fingerprint = public_key_fingerprint("test-key-material");
        assert!(fingerprint.starts_with("sha256:"));
        assert_eq!(fingerprint.len(), 7 + 64);
    }

    #[test]
    fn build_health_checks_includes_dns_and_tcp_targets() {
        let config = default_server_config();
        let checks = build_health_checks(&config);
        assert_eq!(checks.len(), 4);
        assert!(checks
            .iter()
            .any(|check| check.target.starts_with("dns:rd.example.com")));
        assert!(checks
            .iter()
            .any(|check| check.target == "tcp:rd.example.com:21116"));
        assert!(checks
            .iter()
            .any(|check| check.target == "tcp:rd.example.com:21117"));
    }

    #[test]
    fn build_health_checks_uses_explicit_and_ipv6_probe_targets() {
        let mut config = default_server_config();
        config.id_server = "rd.example.com:12345".to_string();
        config.relay_server = "[2001:db8::1]:23456".to_string();
        let checks = build_health_checks(&config);
        assert!(checks
            .iter()
            .any(|check| check.target == "tcp:rd.example.com:12345"));
        assert!(checks
            .iter()
            .any(|check| check.target == "tcp:2001:db8::1:23456"));
        assert!(checks
            .iter()
            .any(|check| check.target == "dns:rd.example.com"));
        assert!(checks.iter().any(|check| check.target == "dns:2001:db8::1"));
        assert!(!checks.iter().any(|check| check.target.ends_with(":21116")));
        assert!(!checks.iter().any(|check| check.target.ends_with(":21117")));
    }

    #[test]
    fn tcp_port_check_reports_failure_for_unreachable_host() {
        let result = tcp_port_check("127.0.0.1", 1, 100);
        assert_eq!(result.status, "failed");
        assert_eq!(result.target, "tcp:127.0.0.1:1");
    }
}
