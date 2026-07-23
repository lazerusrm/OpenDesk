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
    pub client_token_hmac_key: Vec<u8>,
    pub transport_introspection_key: Option<Vec<u8>>,
    pub rustdesk_download_windows_url: Option<String>,
    pub rustdesk_download_macos_url: Option<String>,
    pub rustdesk_download_linux_url: Option<String>,
    pub rustdesk_download_android_url: Option<String>,
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
            client_token_hmac_key: required_client_token_hmac_key(),
            transport_introspection_key: optional_hex_key("OPENDESK_TRANSPORT_INTROSPECTION_KEY"),
            rustdesk_download_windows_url: optional_https_url(
                "OPENDESK_RUSTDESK_DOWNLOAD_WINDOWS_URL",
            ),
            rustdesk_download_macos_url: optional_https_url("OPENDESK_RUSTDESK_DOWNLOAD_MACOS_URL"),
            rustdesk_download_linux_url: optional_https_url("OPENDESK_RUSTDESK_DOWNLOAD_LINUX_URL"),
            rustdesk_download_android_url: optional_https_url(
                "OPENDESK_RUSTDESK_DOWNLOAD_ANDROID_URL",
            ),
        }
    }
}

fn required_client_token_hmac_key() -> Vec<u8> {
    let encoded = env::var("OPENDESK_CLIENT_TOKEN_HMAC_KEY")
        .expect("OPENDESK_CLIENT_TOKEN_HMAC_KEY is required");
    let key = hex::decode(encoded).expect("OPENDESK_CLIENT_TOKEN_HMAC_KEY must be hexadecimal");
    assert!(
        key.len() >= 32,
        "OPENDESK_CLIENT_TOKEN_HMAC_KEY must contain at least 32 bytes"
    );
    key
}

fn optional_hex_key(name: &str) -> Option<Vec<u8>> {
    optional_env(name).map(|encoded| {
        let key = hex::decode(encoded).unwrap_or_else(|_| panic!("{name} must be hexadecimal"));
        assert!(key.len() >= 32, "{name} must contain at least 32 bytes");
        key
    })
}

fn optional_https_url(name: &str) -> Option<String> {
    optional_env(name).map(|value| {
        assert!(value.starts_with("https://"), "{name} must use https");
        value
    })
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
