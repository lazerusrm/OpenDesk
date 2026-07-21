use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub listen_addr: SocketAddr,
    pub database_url: String,
    pub cookie_secure: bool,
    pub bootstrap_admin_username: String,
    pub bootstrap_admin_password: String,
    pub public_base_url: String,
    pub backup_schedule: Option<String>,
    pub backup_destination: Option<PathBuf>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let listen_addr = env::var("OPENDESK_LISTEN_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .expect("valid OPENDESK_LISTEN_ADDR");
        let data_dir = env::var("OPENDESK_DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let database_path = PathBuf::from(&data_dir).join("opendesk.sqlite");
        let database_url = format!("sqlite:{}?mode=rwc", database_path.display());
        Self {
            listen_addr,
            database_url,
            cookie_secure: env::var("OPENDESK_COOKIE_SECURE")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            bootstrap_admin_username: env::var("OPENDESK_BOOTSTRAP_ADMIN_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),
            bootstrap_admin_password: env::var("OPENDESK_BOOTSTRAP_ADMIN_PASSWORD")
                .unwrap_or_else(|_| "change-me".to_string()),
            public_base_url: env::var("OPENDESK_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string()),
            backup_schedule: optional_env("OPENDESK_BACKUP_SCHEDULE"),
            backup_destination: optional_env("OPENDESK_BACKUP_DIR").map(PathBuf::from),
        }
    }
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_env_discards_missing_values() {
        assert_eq!(optional_env("OPENDESK_TEST_MISSING"), None);
    }
}
