use crate::domain::health::host_from_server_value;
use crate::domain::server_config::ServerConfig;

pub const EXPLICIT_HELPER_DEFAULT_PORT: u16 = 21117;

pub fn generate_default_server_helper(rustdesk_id: Option<&str>) -> Option<String> {
    rustdesk_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn generate_explicit_server_helper(
    rustdesk_id: Option<&str>,
    config: &ServerConfig,
    port: u16,
) -> Option<String> {
    let rustdesk_id = rustdesk_id?.trim();
    let id_server = host_from_server_value(&config.id_server);
    let public_key = config.public_key.trim();
    if rustdesk_id.is_empty() || id_server.is_empty() || public_key.is_empty() {
        return None;
    }
    Some(format!("{rustdesk_id}@{id_server}:{port}?key={public_key}"))
}

pub fn explicit_server_helper_for_device(
    rustdesk_id: Option<&str>,
    config: &ServerConfig,
) -> Option<String> {
    generate_explicit_server_helper(rustdesk_id, config, EXPLICIT_HELPER_DEFAULT_PORT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server_config::default_server_config;

    #[test]
    fn default_helper_returns_plain_rustdesk_id() {
        assert_eq!(
            generate_default_server_helper(Some(" 123456789 ")),
            Some("123456789".to_string())
        );
        assert_eq!(generate_default_server_helper(None), None);
    }

    #[test]
    fn explicit_helper_uses_id_server_port_and_key() {
        let mut config = default_server_config();
        config.public_key = "test-public-key".to_string();
        assert_eq!(
            generate_explicit_server_helper(Some("987654321"), &config, 21117),
            Some("987654321@rd.example.com:21117?key=test-public-key".to_string())
        );
    }

    #[test]
    fn explicit_helper_strips_port_suffix_from_id_server() {
        let mut config = default_server_config();
        config.id_server = "rd.example.com:21116".to_string();
        config.public_key = "test-public-key".to_string();
        assert_eq!(
            generate_explicit_server_helper(Some("123456789"), &config, 21117),
            Some("123456789@rd.example.com:21117?key=test-public-key".to_string())
        );
    }

    #[test]
    fn explicit_helper_requires_rustdesk_id_and_key() {
        let config = default_server_config();
        assert_eq!(generate_explicit_server_helper(Some(""), &config, 21117), None);
        let mut missing_key = config.clone();
        missing_key.public_key = "  ".to_string();
        assert_eq!(
            generate_explicit_server_helper(Some("123"), &missing_key, 21117),
            None
        );
    }
}