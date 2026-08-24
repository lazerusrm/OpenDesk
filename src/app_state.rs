use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::login_throttle::LoginThrottle;
use crate::onboard_guard::OnboardGuard;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub data_dir: PathBuf,
    pub cookie_secure: bool,
    pub public_base_url: Arc<Mutex<String>>,
    pub backup_schedule: Option<String>,
    pub backup_destination_configured: bool,
    pub client_token_hmac_key: Vec<u8>,
    pub transport_introspection_key: Option<Vec<u8>>,
    pub rustdesk_download_windows_url: Option<String>,
    pub rustdesk_download_macos_url: Option<String>,
    pub rustdesk_download_linux_url: Option<String>,
    pub rustdesk_download_android_url: Option<String>,
    pub signed_client_dir: Option<PathBuf>,
    pub login_throttle: Arc<Mutex<LoginThrottle>>,
    pub onboard_guard: Arc<Mutex<OnboardGuard>>,
    pub pending_onboard_totp: Arc<Mutex<HashMap<Uuid, Vec<u8>>>>,
}

impl AppState {
    pub fn public_base_url(&self) -> String {
        self.public_base_url
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|error| error.into_inner().clone())
    }

    pub fn set_public_base_url(&self, value: String) {
        if let Ok(mut guard) = self.public_base_url.lock() {
            *guard = value;
        }
    }
}
