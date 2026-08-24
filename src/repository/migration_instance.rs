use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum MigrationInstanceError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("stored instance identity is invalid")]
    InvalidIdentity,
    #[error("stored instance identity is ambiguous")]
    AmbiguousIdentity,
}

pub async fn ensure_instance_uuid(pool: &SqlitePool) -> Result<Uuid, MigrationInstanceError> {
    let existing: Vec<(String,)> = sqlx::query_as("SELECT instance_uuid FROM opendesk_instance")
        .fetch_all(pool)
        .await?;
    match existing.as_slice() {
        [(value,)] => Uuid::parse_str(value).map_err(|_| MigrationInstanceError::InvalidIdentity),
        [] => {
            let instance_uuid = Uuid::new_v4();
            sqlx::query("INSERT INTO opendesk_instance (instance_uuid) VALUES (?)")
                .bind(instance_uuid.to_string())
                .execute(pool)
                .await?;
            Ok(instance_uuid)
        }
        _ => Err(MigrationInstanceError::AmbiguousIdentity),
    }
}
