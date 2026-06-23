use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::audit_event::{redact_audit_detail, AuditEventDraft};
use crate::time_format::format_timestamp;

pub async fn insert_audit_event(
    pool: &SqlitePool,
    draft: &AuditEventDraft,
) -> Result<(), sqlx::Error> {
    let audit_event_uuid = Uuid::new_v4();
    let now = format_timestamp(OffsetDateTime::now_utc());
    let detail_json = draft
        .detail
        .as_ref()
        .map(redact_audit_detail)
        .map(|value| value.to_string());
    sqlx::query(
        "INSERT INTO audit_events (
            audit_event_uuid, actor_user_uuid, action, object_type, object_uuid,
            outcome, source, detail_json, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(audit_event_uuid.to_string())
    .bind(draft.actor_user_uuid.map(|value| value.to_string()))
    .bind(&draft.action)
    .bind(&draft.object_type)
    .bind(draft.object_uuid.map(|value| value.to_string()))
    .bind(&draft.outcome)
    .bind(&draft.source)
    .bind(detail_json)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}