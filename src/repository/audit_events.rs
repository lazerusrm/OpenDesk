use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::audit_event::{redact_audit_detail, AuditEventDraft};
use crate::time_format::format_timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEventRow {
    pub audit_event_uuid: Uuid,
    pub actor_user_uuid: Option<Uuid>,
    pub actor_username: Option<String>,
    pub action: String,
    pub object_type: String,
    pub object_uuid: Option<Uuid>,
    pub outcome: String,
    pub source: String,
    pub detail_json: Option<String>,
    pub created_at: String,
}

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

pub async fn list_audit_events(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<AuditEventRow>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            String,
        ),
    >(
        "SELECT a.audit_event_uuid, a.actor_user_uuid, u.username, a.action, a.object_type,
                a.object_uuid, a.outcome, a.source, a.detail_json, a.created_at
         FROM audit_events a
         LEFT JOIN users u ON u.user_uuid = a.actor_user_uuid
         ORDER BY a.created_at DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                audit_event_uuid,
                actor_user_uuid,
                actor_username,
                action,
                object_type,
                object_uuid,
                outcome,
                source,
                detail_json,
                created_at,
            )| AuditEventRow {
                audit_event_uuid: Uuid::parse_str(&audit_event_uuid).expect("stored uuid"),
                actor_user_uuid: actor_user_uuid
                    .as_deref()
                    .map(Uuid::parse_str)
                    .transpose()
                    .expect("stored uuid"),
                actor_username,
                action,
                object_type,
                object_uuid: object_uuid
                    .as_deref()
                    .map(Uuid::parse_str)
                    .transpose()
                    .expect("stored uuid"),
                outcome,
                source,
                detail_json,
                created_at,
            },
        )
        .collect())
}

/// CSV export of Tier-1 audit rows. Detail values are already redacted at insert time.
pub fn render_audit_events_csv(events: &[AuditEventRow]) -> String {
    let mut out = String::from(
        "created_at,actor_username,action,object_type,object_uuid,outcome,source,detail_json\n",
    );
    for event in events {
        out.push_str(&csv_escape(&event.created_at));
        out.push(',');
        out.push_str(&csv_escape(event.actor_username.as_deref().unwrap_or("")));
        out.push(',');
        out.push_str(&csv_escape(&event.action));
        out.push(',');
        out.push_str(&csv_escape(&event.object_type));
        out.push(',');
        out.push_str(&csv_escape(
            &event
                .object_uuid
                .map(|uuid| uuid.to_string())
                .unwrap_or_default(),
        ));
        out.push(',');
        out.push_str(&csv_escape(&event.outcome));
        out.push(',');
        out.push_str(&csv_escape(&event.source));
        out.push(',');
        out.push_str(&csv_escape(event.detail_json.as_deref().unwrap_or("")));
        out.push('\n');
    }
    out
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_audit_events_csv_includes_header_and_row() {
        let events = vec![AuditEventRow {
            audit_event_uuid: Uuid::nil(),
            actor_user_uuid: None,
            actor_username: Some("admin".to_string()),
            action: "login".to_string(),
            object_type: "session".to_string(),
            object_uuid: None,
            outcome: "success".to_string(),
            source: "web".to_string(),
            detail_json: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }];
        let csv = render_audit_events_csv(&events);
        assert!(csv.starts_with("created_at,actor_username,action,"));
        assert!(csv.contains("admin"));
        assert!(csv.contains("login"));
    }
}
