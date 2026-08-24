use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use time::{Duration, OffsetDateTime};

use crate::app_state::AppState;
use crate::domain::backup::backup_readiness;
use crate::domain::enrollment_token::enrollment_token_is_active;
use crate::domain::health::build_health_checks;
use crate::domain::server_config::default_server_config;
use crate::http::session::require_user;
use crate::http::views::{nav_permissions_for_role, NavPermissions};
use crate::repository::access_groups::list_access_groups;
use crate::repository::audit_events::list_audit_events;
use crate::repository::device_visibility::list_visible_device_uuids_for_user;
use crate::repository::devices::list_devices;
use crate::repository::enrollment_tokens::list_enrollment_tokens;
use crate::repository::server_config::load_server_config;
use crate::repository::users::count_users;
use crate::time_format::parse_timestamp;

const AUDIT_PREVIEW_LIMIT: i64 = 10;
const RECENTLY_SEEN_WINDOW: Duration = Duration::minutes(5);

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub device_count: i64,
    pub archived_count: i64,
    pub recently_seen_count: i64,
    pub user_count: i64,
    pub access_group_count: i64,
    pub address_book_count: i64,
    pub active_enrollment_token_count: i64,
    pub health_pass_count: i64,
    pub health_fail_count: i64,
    pub backup_status: String,
    pub events: Vec<DashboardAuditRow>,
}

#[derive(Clone)]
pub struct DashboardAuditRow {
    pub created_at: String,
    pub actor_display: String,
    pub action: String,
    pub object_type: String,
    pub outcome: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard_page))
        .route("/dashboard", get(dashboard_page))
}

async fn dashboard_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let now = OffsetDateTime::now_utc();
    let recently_seen_cutoff = now - RECENTLY_SEEN_WINDOW;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let visible_device_uuids = list_visible_device_uuids_for_user(&state.db, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let visible_devices: Vec<_> = devices
        .into_iter()
        .filter(|device| visible_device_uuids.contains(&device.device_uuid))
        .collect();
    let device_count = visible_devices.len() as i64;
    let archived_count = visible_devices
        .iter()
        .filter(|device| device.archived)
        .count() as i64;
    // last_checkin_at is heartbeat metadata, not session proof.
    let recently_seen_count = visible_devices
        .iter()
        .filter(|device| is_recently_seen(device.last_checkin_at.as_deref(), recently_seen_cutoff))
        .count() as i64;
    let user_count = count_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let access_group_count = list_access_groups(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .len() as i64;
    let address_book_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM address_books")
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let active_enrollment_token_count = list_enrollment_tokens(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .iter()
        .filter(|token| enrollment_token_is_active(token, now).is_ok())
        .count() as i64;
    let events = list_audit_events(&state.db, AUDIT_PREVIEW_LIMIT)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_iter()
        .map(|event| DashboardAuditRow {
            created_at: event.created_at,
            actor_display: event.actor_username.unwrap_or_else(|| "-".to_string()),
            action: event.action,
            object_type: event.object_type,
            outcome: event.outcome,
        })
        .collect();
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    let health_checks = build_health_checks(&config);
    let health_pass_count = health_checks
        .iter()
        .filter(|check| check.status == "ok")
        .count() as i64;
    let health_fail_count = health_checks
        .iter()
        .filter(|check| check.status == "failed")
        .count() as i64;
    let readiness = backup_readiness(
        state.backup_schedule.is_some(),
        state.backup_destination_configured,
    );
    let view = DashboardView {
        title: "Overview".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        device_count,
        archived_count,
        recently_seen_count,
        user_count,
        access_group_count,
        address_book_count,
        active_enrollment_token_count,
        health_pass_count,
        health_fail_count,
        backup_status: readiness.status.to_string(),
        events,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

fn is_recently_seen(last_checkin_at: Option<&str>, cutoff: OffsetDateTime) -> bool {
    last_checkin_at
        .and_then(parse_timestamp)
        .map(|timestamp| timestamp >= cutoff)
        .unwrap_or(false)
}
