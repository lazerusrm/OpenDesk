use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub cookie_secure: bool,
    pub public_base_url: String,
    pub backup_schedule: Option<String>,
    pub backup_destination_configured: bool,
}
