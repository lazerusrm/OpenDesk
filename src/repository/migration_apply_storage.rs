use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::domain::migration_contract::SanitizedMigrationExport;

use super::MigrationApplyError;

pub(super) async fn reject_collision(
    connection: &mut SqliteConnection,
    table: &str,
    column: &str,
    value: &str,
) -> Result<(), MigrationApplyError> {
    let query = format!("SELECT 1 FROM {table} WHERE {column} = ? LIMIT 1");
    if sqlx::query(&query)
        .bind(value)
        .fetch_optional(&mut *connection)
        .await?
        .is_some()
    {
        return Err(MigrationApplyError::Collision);
    }
    Ok(())
}

pub(super) async fn insert_user(
    connection: &mut SqliteConnection,
    user_uuid: Uuid,
    username: &str,
    role: &str,
    timestamp: &str,
) -> Result<(), sqlx::Error> {
    let disabled_hash = format!("migration-disabled-{}", Uuid::new_v4().simple());
    sqlx::query("INSERT INTO users (user_uuid, username, password_hash, role, activation_state, created_at, updated_at) VALUES (?, ?, ?, ?, 'disabled', ?, ?)")
        .bind(user_uuid.to_string()).bind(username).bind(disabled_hash).bind(role).bind(timestamp).bind(timestamp)
        .execute(&mut *connection).await?;
    Ok(())
}

pub(super) async fn bind(
    connection: &mut SqliteConnection,
    export: &SanitizedMigrationExport,
    kind: &str,
    source_id: &str,
    target_uuid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO migration_source_bindings (source_system, source_instance, entity_kind, source_id, target_uuid, run_id) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&export.provenance.source_system).bind(&export.provenance.source_instance).bind(kind).bind(source_id)
        .bind(target_uuid.to_string()).bind(export.run.run_id.to_string()).execute(&mut *connection).await?;
    Ok(())
}
