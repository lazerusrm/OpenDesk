use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::server_config::ServerConfig;
use crate::time_format::format_timestamp;

pub async fn load_server_config(pool: &SqlitePool) -> Result<Option<ServerConfig>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id_server, relay_server, api_server, public_key
         FROM server_configs ORDER BY updated_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| ServerConfig {
        id_server: row.0,
        relay_server: row.1,
        api_server: row.2,
        public_key: row.3,
    }))
}

pub async fn save_server_config(
    pool: &SqlitePool,
    config: &ServerConfig,
    updated_by_user_uuid: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    let server_config_uuid = Uuid::new_v4();
    let now = format_timestamp(OffsetDateTime::now_utc());
    sqlx::query(
        "INSERT INTO server_configs (
            server_config_uuid, id_server, relay_server, api_server, public_key,
            updated_at, updated_by_user_uuid
         ) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(server_config_uuid.to_string())
    .bind(&config.id_server)
    .bind(&config.relay_server)
    .bind(&config.api_server)
    .bind(&config.public_key)
    .bind(&now)
    .bind(updated_by_user_uuid.map(|value| value.to_string()))
    .execute(pool)
    .await?;
    Ok(())
}