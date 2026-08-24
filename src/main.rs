use std::fs;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use opendesk::{build_router, AppConfig, AppState};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("opendesk=info".parse()?))
        .init();

    let config = AppConfig::from_env()?;
    fs::create_dir_all(&config.data_dir)?;

    let connect_options = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(8))
        .foreign_keys(true);
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    opendesk::repository::migration_instance::ensure_instance_uuid(&db).await?;

    let state = AppState {
        db,
        data_dir: config.data_dir,
        cookie_secure: config.cookie_secure,
        public_base_url: std::sync::Arc::new(std::sync::Mutex::new(config.public_base_url)),
        backup_schedule: config.backup_schedule,
        backup_destination_configured: config.backup_destination.is_some(),
        client_token_hmac_key: config.client_token_hmac_key,
        transport_introspection_key: config.transport_introspection_key,
        rustdesk_download_windows_url: config.rustdesk_download_windows_url,
        rustdesk_download_macos_url: config.rustdesk_download_macos_url,
        rustdesk_download_linux_url: config.rustdesk_download_linux_url,
        rustdesk_download_android_url: config.rustdesk_download_android_url,
        signed_client_dir: config.signed_client_dir,
        login_throttle: std::sync::Arc::new(std::sync::Mutex::new(
            opendesk::login_throttle::LoginThrottle::default(),
        )),
        onboard_guard: std::sync::Arc::new(std::sync::Mutex::new(
            opendesk::onboard_guard::OnboardGuard::default(),
        )),
        pending_onboard_totp: std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashMap::new(),
        )),
    };
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    tracing::info!("opendesk listening on {}", config.listen_addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
