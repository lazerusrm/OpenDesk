use std::fs;
use std::net::SocketAddr;

use opendesk::{build_router, AppConfig, AppState};
use sqlx::sqlite::SqlitePoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("opendesk=info".parse()?))
        .init();

    let config = AppConfig::from_env();
    let data_dir = std::env::var("OPENDESK_DATA_DIR").unwrap_or_else(|_| "data".to_string());
    fs::create_dir_all(&data_dir)?;

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    bootstrap_admin(&db, &config).await?;

    let state = AppState {
        db,
        cookie_secure: config.cookie_secure,
        public_base_url: config.public_base_url,
    };
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    tracing::info!("opendesk listening on {}", config.listen_addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

async fn bootstrap_admin(db: &sqlx::SqlitePool, config: &AppConfig) -> anyhow::Result<()> {
    use opendesk::repository::users::{count_users, create_user};
    if count_users(db).await? > 0 {
        return Ok(());
    }
    create_user(
        db,
        &config.bootstrap_admin_username,
        &config.bootstrap_admin_password,
        "admin",
    )
    .await?;
    tracing::info!(
        "bootstrapped admin user {:?}",
        config.bootstrap_admin_username
    );
    Ok(())
}