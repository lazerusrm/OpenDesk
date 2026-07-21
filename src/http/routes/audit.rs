use askama::Template;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::http::session::require_user;
use crate::http::views::{AuditEventRowView, AuditLogView};
use crate::repository::audit_events::{
    insert_audit_event, list_audit_events, render_audit_events_csv,
};

const AUDIT_LIST_LIMIT: i64 = 500;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/audit", get(audit_list_page))
        .route("/audit/export.csv", get(audit_export_csv))
}

async fn audit_list_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let events = list_audit_events(&state.db, AUDIT_LIST_LIMIT)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let rows = events
        .into_iter()
        .map(|event| AuditEventRowView {
            created_at: event.created_at,
            actor_display: event.actor_username.unwrap_or_else(|| "-".to_string()),
            action: event.action,
            object_type: event.object_type,
            object_uuid_display: event
                .object_uuid
                .map(|uuid| uuid.to_string())
                .unwrap_or_else(|| "-".to_string()),
            outcome: event.outcome,
            source: event.source,
            detail_display: event.detail_json.unwrap_or_else(|| "-".to_string()),
        })
        .collect();
    let view = AuditLogView {
        title: "Audit Log".to_string(),
        show_nav: true,
        csrf_token: user.csrf_token.clone(),
        events: rows,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn audit_export_csv(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let events = list_audit_events(&state.db, AUDIT_LIST_LIMIT)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let csv = render_audit_events_csv(&events);
    // Detail JSON is redacted at insert time; refuse export if raw markers slipped through.
    if csv_contains_raw_secret(&csv) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "audit_export".to_string(),
        object_type: "audit_events".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"audit-events.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

fn csv_contains_raw_secret(csv: &str) -> bool {
    // Refuse if export still contains a non-redacted enrollment_token JSON value.
    csv.contains("\"enrollment_token\":\"") && !csv.contains("\"enrollment_token\":\"[redacted]\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_token_detail_is_allowed() {
        assert!(!csv_contains_raw_secret(
            r#"created_at,actor_username,action,object_type,object_uuid,outcome,source,detail_json
2026-01-01T00:00:00Z,admin,endpoint_checkin,device,,failure,api,"{""enrollment_token"":""[redacted]""}"
"#
        ));
    }
}
