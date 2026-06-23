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
                .unwrap_or(false),
            bootstrap_admin_username: env::var("OPENDESK_BOOTSTRAP_ADMIN_USERNAME")
                .unwrap_or_else(|_| "admin".to_string()),
            bootstrap_admin_password: env::var("OPENDESK_BOOTSTRAP_ADMIN_PASSWORD")
                .unwrap_or_else(|_| "change-me".to_string()),
        }
    }
}