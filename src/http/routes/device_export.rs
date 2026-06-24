use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::device_csv::render_devices_csv;
use crate::domain::device_list::DeviceSearchQuery;
use crate::http::session::{require_user, AuthenticatedUser};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::devices::list_devices;
use crate::repository::sites::list_sites;
use crate::repository::tags::list_device_tag_names_map;

#[derive(Deserialize)]
struct ExportSearchQuery {
    term: Option<String>,
}

pub fn export_csv_href(search_term: &str) -> String {
    let trimmed = search_term.trim();
    if trimmed.is_empty() {
        "/devices/export.csv".to_string()
    } else {
        let encoded = trimmed
            .chars()
            .map(|ch| match ch {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => ch.to_string(),
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", ch as u32),
            })
            .collect::<String>();
        format!("/devices/export.csv?term={encoded}")
    }
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/devices/export.csv", get(devices_csv_export))
}

async fn devices_csv_export(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<ExportSearchQuery>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let search = DeviceSearchQuery {
        term: query.term.unwrap_or_default(),
    };
    let sites = list_sites(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let site_names: std::collections::HashMap<uuid::Uuid, String> = sites
        .iter()
        .map(|site| (site.site_uuid, site.name.clone()))
        .collect();
    let device_tag_names = list_device_tag_names_map(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let csv = render_devices_csv(&devices, &search, &site_names, &device_tag_names);
    write_csv_export_audit(&state, &user).await;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"devices.csv\"",
            ),
        ],
        csv,
    )
        .into_response())
}

async fn write_csv_export_audit(state: &AppState, user: &AuthenticatedUser) {
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "device_csv_export".to_string(),
        object_type: "devices".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}