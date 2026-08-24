use askama::Template;
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{
    nav_permissions_for_role, AuditEventRowView, AuditKindTabView, AuditLogView,
};
use crate::repository::audit_events::{
    insert_audit_event, list_audit_events, render_audit_events_csv, AuditEventRow,
};

const AUDIT_LIST_LIMIT: i64 = 500;
const DETAIL_DISPLAY_CHARS: usize = 80;
const CONNECTION_DISCLAIMER: &str = "Client connection events are reports from official clients. They are not proof that OpenDesk enforced a session.";

fn audit_list_window_notice() -> String {
    format!(
        "This view and CSV export show the newest {AUDIT_LIST_LIMIT} events. Older events remain stored but are not listed or exported."
    )
}

fn audit_export_window_comment() -> String {
    format!("# Newest {AUDIT_LIST_LIMIT} events only; not a complete history.\n")
}

const AUDIT_KINDS: [AuditKind; 5] = [
    AuditKind::All,
    AuditKind::Console,
    AuditKind::Connection,
    AuditKind::File,
    AuditKind::Alarm,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditKind {
    All,
    Console,
    Connection,
    File,
    Alarm,
}

impl AuditKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Console => "console",
            Self::Connection => "connection",
            Self::File => "file",
            Self::Alarm => "alarm",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Console => "Console",
            Self::Connection => "Connection",
            Self::File => "File",
            Self::Alarm => "Alarm",
        }
    }

    fn intro(self) -> &'static str {
        match self {
            Self::All | Self::Console => {
                "Tier-1 operational events (auth, device mutations, enrollment, deployment artifacts, exports). Secret material is redacted."
            }
            Self::Connection => CONNECTION_DISCLAIMER,
            Self::File => "Client file events are reports from official clients.",
            Self::Alarm => "OpenDesk does not ingest Pro alarm tables.",
        }
    }

    fn empty_message(self) -> &'static str {
        match self {
            Self::All => "No audit events.",
            Self::Console => "No console audit events.",
            Self::Connection => "No client connection reports.",
            Self::File => "No client file reports.",
            Self::Alarm => {
                "OpenDesk does not ingest Pro alarm tables. This tab stays empty until an OpenDesk alarm source exists."
            }
        }
    }

    fn connection_disclaimer(self) -> Option<&'static str> {
        match self {
            Self::All => Some(CONNECTION_DISCLAIMER),
            _ => None,
        }
    }

    fn matches(self, action: &str) -> bool {
        match self {
            Self::All => true,
            Self::Console => is_console_action(action),
            Self::Connection => action.starts_with("client_connection_"),
            Self::File => action.starts_with("client_file_"),
            Self::Alarm => false,
        }
    }

    fn href(self) -> String {
        format!("/audit?kind={}", self.as_str())
    }

    fn export_csv_href(self, csrf_token: &str) -> String {
        match self {
            Self::All => format!("/audit/export.csv?csrf_token={csrf_token}"),
            other => format!(
                "/audit/export.csv?kind={}&csrf_token={csrf_token}",
                other.as_str()
            ),
        }
    }
}

fn is_console_action(action: &str) -> bool {
    !action.starts_with("client_connection_")
        && !action.starts_with("client_file_")
        && action != "client_session_record"
}

fn parse_audit_kind(value: Option<&str>) -> Result<AuditKind, ()> {
    match value {
        None => Ok(AuditKind::All),
        Some("all") => Ok(AuditKind::All),
        Some("console") => Ok(AuditKind::Console),
        Some("connection") => Ok(AuditKind::Connection),
        Some("file") => Ok(AuditKind::File),
        Some("alarm") => Ok(AuditKind::Alarm),
        Some(_) => Err(()),
    }
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    kind: Option<String>,
    csrf_token: Option<String>,
}

fn audit_kind_from_query(query: &AuditQuery) -> Result<AuditKind, Response> {
    parse_audit_kind(query.kind.as_deref()).map_err(|_| StatusCode::BAD_REQUEST.into_response())
}

fn kind_tabs(selected: AuditKind) -> Vec<AuditKindTabView> {
    AUDIT_KINDS
        .iter()
        .copied()
        .map(|kind| AuditKindTabView {
            label: kind.label().to_string(),
            href: kind.href(),
            selected: kind == selected,
        })
        .collect()
}

fn filter_events(events: Vec<AuditEventRow>, kind: AuditKind) -> Vec<AuditEventRow> {
    events
        .into_iter()
        .filter(|event| kind.matches(&event.action))
        .collect()
}

fn truncated_detail(detail_json: Option<String>) -> (String, String) {
    let full = detail_json.unwrap_or_else(|| "-".to_string());
    let mut chars = full.chars();
    let mut display: String = chars.by_ref().take(DETAIL_DISPLAY_CHARS).collect();
    if chars.next().is_some() {
        display.push('…');
    }
    (display, full)
}

fn event_row(event: AuditEventRow) -> AuditEventRowView {
    let (detail_display, detail_title) = truncated_detail(event.detail_json);
    AuditEventRowView {
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
        detail_display,
        detail_title,
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/audit", get(audit_list_page))
        .route("/audit/export.csv", get(audit_export_csv))
}

async fn audit_list_page(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<AuditQuery>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::AuditView).await?;
    let kind = audit_kind_from_query(&query)?;
    // Kind filters apply to the newest AUDIT_LIST_LIMIT rows; they are not a separate query.
    let events = list_audit_events(&state.db, AUDIT_LIST_LIMIT)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let rows: Vec<AuditEventRowView> = filter_events(events, kind)
        .into_iter()
        .map(event_row)
        .collect();
    let has_events = !rows.is_empty();
    let view = AuditLogView {
        title: "Audit Log".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        events: rows,
        tabs: kind_tabs(kind),
        export_csv_href: kind.export_csv_href(&user.csrf_token),
        intro: kind.intro().to_string(),
        connection_disclaimer: kind.connection_disclaimer().map(str::to_string),
        list_window_notice: audit_list_window_notice(),
        empty_message: kind.empty_message().to_string(),
        has_events,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn audit_export_csv(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<AuditQuery>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::AuditExport).await?;
    require_csrf(&user, query.csrf_token.as_deref().unwrap_or(""))?;
    let kind = audit_kind_from_query(&query)?;
    let events = list_audit_events(&state.db, AUDIT_LIST_LIMIT)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let events = filter_events(events, kind);
    let csv = format!(
        "{}{}",
        audit_export_window_comment(),
        render_audit_events_csv(&events)
    );
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
    fn omitted_kind_defaults_to_all() {
        assert_eq!(parse_audit_kind(None), Ok(AuditKind::All));
        assert_eq!(parse_audit_kind(Some("all")), Ok(AuditKind::All));
    }

    #[test]
    fn unknown_kind_is_rejected() {
        assert_eq!(parse_audit_kind(Some("")), Err(()));
        assert_eq!(parse_audit_kind(Some("sessions")), Err(()));
        assert_eq!(parse_audit_kind(Some("connect")), Err(()));
        assert_eq!(parse_audit_kind(Some("All")), Err(()));
    }

    #[test]
    fn console_kind_excludes_client_reported_actions() {
        assert!(AuditKind::Console.matches("login"));
        assert!(AuditKind::Console.matches("enrollment_token_create"));
        assert!(!AuditKind::Console.matches("client_connection_open"));
        assert!(!AuditKind::Console.matches("client_file_transfer"));
        assert!(!AuditKind::Console.matches("client_session_record"));
    }

    #[test]
    fn connection_and_file_kinds_use_action_prefixes() {
        assert!(AuditKind::Connection.matches("client_connection_open"));
        assert!(!AuditKind::Connection.matches("client_connection"));
        assert!(!AuditKind::Connection.matches("login"));
        assert!(AuditKind::File.matches("client_file_transfer"));
        assert!(!AuditKind::File.matches("client_file"));
        assert!(!AuditKind::File.matches("client_connection_open"));
    }

    #[test]
    fn alarm_kind_matches_nothing() {
        assert!(!AuditKind::Alarm.matches("alarm"));
        assert!(!AuditKind::Alarm.matches("client_connection_open"));
        assert!(!AuditKind::Alarm.matches("login"));
    }

    #[test]
    fn all_kind_keeps_current_list() {
        assert!(AuditKind::All.matches("login"));
        assert!(AuditKind::All.matches("client_connection_open"));
        assert!(AuditKind::All.matches("client_file_transfer"));
        assert!(AuditKind::All.matches("client_session_record"));
    }

    #[test]
    fn list_and_export_disclose_newest_event_window() {
        assert!(audit_list_window_notice().contains("newest 500 events"));
        assert!(audit_list_window_notice().contains("not listed or exported"));
        let comment = audit_export_window_comment();
        assert!(comment.starts_with("# Newest 500 events only"));
        assert!(comment.contains("not a complete history"));
    }

    #[test]
    fn export_href_omits_kind_for_all() {
        assert_eq!(
            AuditKind::All.export_csv_href("abc"),
            "/audit/export.csv?csrf_token=abc"
        );
        assert_eq!(
            AuditKind::Console.export_csv_href("abc"),
            "/audit/export.csv?kind=console&csrf_token=abc"
        );
    }

    #[test]
    fn truncated_detail_keeps_full_title() {
        let long = "a".repeat(DETAIL_DISPLAY_CHARS + 4);
        let (display, title) = truncated_detail(Some(long.clone()));
        assert_eq!(title, long);
        assert!(display.ends_with('…'));
        assert_eq!(display.chars().count(), DETAIL_DISPLAY_CHARS + 1);
        let (short_display, short_title) = truncated_detail(None);
        assert_eq!(short_display, "-");
        assert_eq!(short_title, "-");
    }

    #[test]
    fn redacted_token_detail_is_allowed() {
        assert!(!csv_contains_raw_secret(
            r#"created_at,actor_username,action,object_type,object_uuid,outcome,source,detail_json
2026-01-01T00:00:00Z,admin,endpoint_checkin,device,,failure,api,"{""enrollment_token"":""[redacted]""}"
"#
        ));
    }
}
