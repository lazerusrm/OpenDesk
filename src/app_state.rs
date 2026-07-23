use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub cookie_secure: bool,
    pub public_base_url: String,
    pub backup_schedule: Option<String>,
    pub backup_destination_configured: bool,
    pub client_token_hmac_key: Vec<u8>,
    pub transport_introspection_key: Option<Vec<u8>>,
    pub rustdesk_download_windows_url: Option<String>,
    pub rustdesk_download_macos_url: Option<String>,
    pub rustdesk_download_linux_url: Option<String>,
    pub rustdesk_download_android_url: Option<String>,
}
