use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use rand::RngCore;
use thiserror::Error;

pub const HMAC_KEY_FILE: &str = "opendesk.hmac";
pub const PUBLIC_BASE_URL_FILE: &str = "opendesk.public_base_url";
pub const SQLITE_FILE: &str = "opendesk.sqlite";
const HMAC_KEY_BYTES: usize = 32;
const DEFAULT_PUBLIC_BASE_URL: &str = "http://127.0.0.1:8080";
// Linux O_NOFOLLOW (0400000). Applies only to the last path component.
const O_NOFOLLOW: i32 = 0x20000;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub listen_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub database_url: String,
    pub cookie_secure: bool,
    pub public_base_url: String,
    pub backup_schedule: Option<String>,
    pub backup_destination: Option<PathBuf>,
    pub client_token_hmac_key: Vec<u8>,
    pub transport_introspection_key: Option<Vec<u8>>,
    pub rustdesk_download_windows_url: Option<String>,
    pub rustdesk_download_macos_url: Option<String>,
    pub rustdesk_download_linux_url: Option<String>,
    pub rustdesk_download_android_url: Option<String>,
    pub signed_client_dir: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("client token HMAC key is missing for an existing database")]
    MissingHmacForExistingDatabase,
    #[error("client token HMAC key must be hexadecimal")]
    InvalidHmacEncoding,
    #[error("client token HMAC key must contain at least {HMAC_KEY_BYTES} bytes")]
    HmacTooShort,
    #[error("failed to persist client token HMAC key")]
    HmacPersist(#[from] io::Error),
    #[error("invalid OPENDESK_LISTEN_ADDR")]
    InvalidListenAddr,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let listen_addr = env::var("OPENDESK_LISTEN_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse()
            .map_err(|_| ConfigError::InvalidListenAddr)?;
        let data_dir =
            PathBuf::from(env::var("OPENDESK_DATA_DIR").unwrap_or_else(|_| "data".into()));
        let database_path = data_dir.join(SQLITE_FILE);
        let database_url = format!("sqlite:{}?mode=rwc", database_path.display());
        let env_hmac = optional_env("OPENDESK_CLIENT_TOKEN_HMAC_KEY");
        Ok(Self {
            listen_addr,
            data_dir: data_dir.clone(),
            database_url,
            cookie_secure: env::var("OPENDESK_COOKIE_SECURE")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            public_base_url: resolve_public_base_url(
                &data_dir,
                optional_env("OPENDESK_PUBLIC_BASE_URL").as_deref(),
            ),
            backup_schedule: optional_env("OPENDESK_BACKUP_SCHEDULE"),
            backup_destination: optional_env("OPENDESK_BACKUP_DIR").map(PathBuf::from),
            client_token_hmac_key: resolve_client_token_hmac_key(&data_dir, env_hmac.as_deref())?,
            transport_introspection_key: optional_hex_key("OPENDESK_TRANSPORT_INTROSPECTION_KEY"),
            rustdesk_download_windows_url: optional_https_url(
                "OPENDESK_RUSTDESK_DOWNLOAD_WINDOWS_URL",
            ),
            rustdesk_download_macos_url: optional_https_url("OPENDESK_RUSTDESK_DOWNLOAD_MACOS_URL"),
            rustdesk_download_linux_url: optional_https_url("OPENDESK_RUSTDESK_DOWNLOAD_LINUX_URL"),
            rustdesk_download_android_url: optional_https_url(
                "OPENDESK_RUSTDESK_DOWNLOAD_ANDROID_URL",
            ),
            signed_client_dir: optional_existing_dir("OPENDESK_SIGNED_CLIENT_DIR"),
        })
    }
}

pub fn resolve_client_token_hmac_key(
    data_dir: &Path,
    env_hex: Option<&str>,
) -> Result<Vec<u8>, ConfigError> {
    if let Some(encoded) = env_hex.map(str::trim).filter(|value| !value.is_empty()) {
        return decode_hmac_hex(encoded);
    }
    fs::create_dir_all(data_dir)?;
    let key_path = data_dir.join(HMAC_KEY_FILE);
    match read_nofollow_file(&key_path) {
        Ok(bytes) => decode_hmac_hex(String::from_utf8_lossy(&bytes).trim()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if data_dir.join(SQLITE_FILE).is_file() {
                return Err(ConfigError::MissingHmacForExistingDatabase);
            }
            let mut key = vec![0u8; HMAC_KEY_BYTES];
            rand::thread_rng().fill_bytes(&mut key);
            match persist_hmac_file(&key_path, &key) {
                Ok(()) => Ok(key),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    let bytes = read_nofollow_file(&key_path)?;
                    decode_hmac_hex(String::from_utf8_lossy(&bytes).trim())
                }
                Err(error) => Err(error.into()),
            }
        }
        Err(error) => Err(error.into()),
    }
}

pub fn nofollow_regular_file_exists(path: &Path) -> bool {
    open_nofollow_regular(path).is_ok()
}

pub fn read_nofollow_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = open_nofollow_regular(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn open_nofollow_regular(path: &Path) -> io::Result<fs::File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    Ok(file)
}

pub fn resolve_public_base_url(data_dir: &Path, env_value: Option<&str>) -> String {
    if let Some(value) = env_value.map(str::trim).filter(|value| !value.is_empty()) {
        return value.to_string();
    }
    fs::read_to_string(data_dir.join(PUBLIC_BASE_URL_FILE))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PUBLIC_BASE_URL.to_string())
}

pub fn persist_public_base_url(data_dir: &Path, url: &str) -> Result<(), io::Error> {
    fs::create_dir_all(data_dir)?;
    fs::write(
        data_dir.join(PUBLIC_BASE_URL_FILE),
        format!("{}\n", url.trim()),
    )
}

pub fn public_base_url_is_valid(value: &str) -> bool {
    let value = value.trim();
    (value.starts_with("http://") || value.starts_with("https://"))
        && crate::http::session::origin_from_url(value).is_some()
}

fn decode_hmac_hex(encoded: &str) -> Result<Vec<u8>, ConfigError> {
    let key = hex::decode(encoded).map_err(|_| ConfigError::InvalidHmacEncoding)?;
    if key.len() < HMAC_KEY_BYTES {
        return Err(ConfigError::HmacTooShort);
    }
    Ok(key)
}

fn persist_hmac_file(path: &Path, key: &[u8]) -> Result<(), io::Error> {
    let mut nonce = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut nonce);
    let tmp = path.with_file_name(format!("{}.{}.tmp", HMAC_KEY_FILE, hex::encode(nonce)));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(O_NOFOLLOW)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(hex::encode(key).as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        // hard_link fails if the destination name exists, so a complete file
        // cannot be replaced by a concurrent first-boot write.
        fs::hard_link(&tmp, path)
    })();
    let _ = fs::remove_file(&tmp);
    result
}

fn optional_hex_key(name: &str) -> Option<Vec<u8>> {
    optional_env(name).map(|encoded| {
        let key = hex::decode(encoded).unwrap_or_else(|_| panic!("{name} must be hexadecimal"));
        assert!(
            key.len() >= HMAC_KEY_BYTES,
            "{name} must contain at least {HMAC_KEY_BYTES} bytes"
        );
        key
    })
}

fn optional_https_url(name: &str) -> Option<String> {
    optional_env(name).map(|value| {
        assert!(value.starts_with("https://"), "{name} must use https");
        value
    })
}

fn optional_existing_dir(name: &str) -> Option<PathBuf> {
    optional_env(name).map(|value| {
        let path = PathBuf::from(value);
        assert!(path.is_dir(), "{name} must be an existing directory");
        path
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
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("opendesk-hmac-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("temp dir");
        path
    }

    #[test]
    fn optional_env_discards_missing_values() {
        assert_eq!(optional_env("OPENDESK_TEST_MISSING"), None);
    }

    #[test]
    fn hmac_generates_at_least_32_bytes_and_reuses_file_without_env() {
        let dir = temp_dir();
        let first = resolve_client_token_hmac_key(&dir, None).expect("generate");
        assert!(first.len() >= HMAC_KEY_BYTES);
        let stored = fs::read_to_string(dir.join(HMAC_KEY_FILE)).expect("hmac file");
        assert!(hex::decode(stored.trim()).expect("hex").len() >= HMAC_KEY_BYTES);
        let second = resolve_client_token_hmac_key(&dir, None).expect("reuse");
        assert_eq!(first, second);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hmac_env_overrides_missing_file() {
        let dir = temp_dir();
        let encoded = "aa".repeat(HMAC_KEY_BYTES);
        let key = resolve_client_token_hmac_key(&dir, Some(&encoded)).expect("env");
        assert_eq!(key, vec![0xaa; HMAC_KEY_BYTES]);
        assert!(!dir.join(HMAC_KEY_FILE).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hmac_fails_closed_when_database_exists_without_persisted_or_env_key() {
        let dir = temp_dir();
        fs::write(dir.join(SQLITE_FILE), b"").expect("sqlite");
        let error = resolve_client_token_hmac_key(&dir, None).expect_err("missing");
        assert!(matches!(error, ConfigError::MissingHmacForExistingDatabase));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn public_base_url_file_is_used_when_env_is_unset() {
        let dir = temp_dir();
        persist_public_base_url(&dir, "https://rd.example.com").expect("persist");
        assert_eq!(
            resolve_public_base_url(&dir, None),
            "https://rd.example.com"
        );
        assert_eq!(
            resolve_public_base_url(&dir, Some("https://other.example.com")),
            "https://other.example.com"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hmac_rejects_symlink_path() {
        let dir = temp_dir();
        let target = dir.join("other.hmac");
        fs::write(&target, format!("{}\n", "ab".repeat(HMAC_KEY_BYTES))).expect("target");
        std::os::unix::fs::symlink(&target, dir.join(HMAC_KEY_FILE)).expect("symlink");
        assert!(resolve_client_token_hmac_key(&dir, None).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hmac_exclusive_create_does_not_replace_a_distinct_key() {
        let dir = temp_dir();
        let first = std::thread::scope(|scope| {
            let left = dir.clone();
            let right = dir.clone();
            let a = scope.spawn(move || resolve_client_token_hmac_key(&left, None));
            let b = scope.spawn(move || resolve_client_token_hmac_key(&right, None));
            let a = a.join().expect("thread").expect("left");
            let b = b.join().expect("thread").expect("right");
            assert_eq!(a, b);
            a
        });
        let again = resolve_client_token_hmac_key(&dir, None).expect("reuse");
        assert_eq!(first, again);
        let _ = fs::remove_dir_all(&dir);
    }
}
