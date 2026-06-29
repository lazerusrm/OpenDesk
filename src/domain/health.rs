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

pub fn host_from_server_value(value: &str) -> String {
    value.split(':').next().unwrap_or(value).trim().to_string()
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
    match TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_millis(timeout_ms),
    ) {
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
        tcp_port_check(&config.id_server, HBBS_TCP_PORT, timeout),
        tcp_port_check(&config.relay_server, HBBR_TCP_PORT, timeout),
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
        assert!(checks.iter().any(|check| check.target.starts_with("dns:rd.example.com")));
        assert!(checks
            .iter()
            .any(|check| check.target == "tcp:rd.example.com:21116"));
        assert!(checks
            .iter()
            .any(|check| check.target == "tcp:rd.example.com:21117"));
    }

    #[test]
    fn tcp_port_check_reports_failure_for_unreachable_host() {
        let result = tcp_port_check("127.0.0.1", 1, 100);
        assert_eq!(result.status, "failed");
        assert_eq!(result.target, "tcp:127.0.0.1:1");
    }
}