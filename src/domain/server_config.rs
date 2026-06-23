use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub id_server: String,
    pub relay_server: String,
    pub api_server: String,
    pub public_key: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServerConfigValidationError {
    #[error("id_server must not be empty")]
    EmptyIdServer,
    #[error("relay_server must not be empty")]
    EmptyRelayServer,
}

pub fn validate_server_config(config: &ServerConfig) -> Result<(), ServerConfigValidationError> {
    if config.id_server.trim().is_empty() {
        return Err(ServerConfigValidationError::EmptyIdServer);
    }
    if config.relay_server.trim().is_empty() {
        return Err(ServerConfigValidationError::EmptyRelayServer);
    }
    Ok(())
}

pub fn default_server_config() -> ServerConfig {
    ServerConfig {
        id_server: "rd.example.com".to_string(),
        relay_server: "rd.example.com".to_string(),
        api_server: String::new(),
        public_key: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_server_config_requires_id_server() {
        let config = ServerConfig {
            id_server: " ".to_string(),
            relay_server: "rd.example.com".to_string(),
            api_server: String::new(),
            public_key: String::new(),
        };
        assert_eq!(
            validate_server_config(&config),
            Err(ServerConfigValidationError::EmptyIdServer)
        );
    }

    #[test]
    fn default_server_config_uses_placeholders() {
        let config = default_server_config();
        assert_eq!(config.id_server, "rd.example.com");
        assert_eq!(config.relay_server, "rd.example.com");
    }
}