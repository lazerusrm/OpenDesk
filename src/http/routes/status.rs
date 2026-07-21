use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;
use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::domain::backup::backup_readiness;
use crate::domain::health::{build_health_checks, public_key_fingerprint};
use crate::domain::server_config::default_server_config;
use crate::http::session::require_user;
use crate::http::views::{HealthCheckRowView, StatusView};
use crate::repository::server_config::load_server_config;
use crate::time_format::format_timestamp;

#[derive(Debug, Serialize)]
struct DiagnosticCheck {
    name: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticBackup {
    status: &'static str,
    schedule_configured: bool,
    destination_configured: bool,
    execution: &'static str,
}

#[derive(Debug, Serialize)]
struct DiagnosticResponse {
    status: &'static str,
    checked_at: String,
    opendesk: &'static str,
    database: &'static str,
    rustdesk: &'static str,
    checks: Vec<DiagnosticCheck>,
    backup: DiagnosticBackup,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(status_page))
        .route("/status/diagnostics.json", get(diagnostics_json))
}

async fn status_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    let checks = build_health_checks(&config)
        .into_iter()
        .map(|check| HealthCheckRowView {
            label: check.label,
            target: check.target,
            status: check.status,
            detail: check.detail,
        })
        .collect();
    let readiness = backup_readiness(
        state.backup_schedule.is_some(),
        state.backup_destination_configured,
    );
    let view = StatusView {
        title: "Status".to_string(),
        show_nav: true,
        csrf_token: user.csrf_token.clone(),
        id_server: config.id_server.clone(),
        relay_server: config.relay_server.clone(),
        public_key_fingerprint: public_key_fingerprint(&config.public_key),
        backup_status: readiness.status.to_string(),
        backup_execution: readiness.execution.to_string(),
        checks,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn diagnostics_json(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<DiagnosticResponse>, Response> {
    let _user = require_user(&state, &jar).await?;
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    let health_checks = build_health_checks(&config);
    let rustdesk_ok = health_checks.iter().all(|check| check.status == "ok");
    let database_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();
    let readiness = backup_readiness(
        state.backup_schedule.is_some(),
        state.backup_destination_configured,
    );
    let status = if database_ok && rustdesk_ok {
        "ok"
    } else {
        "degraded"
    };
    Ok(Json(DiagnosticResponse {
        status,
        checked_at: format_timestamp(OffsetDateTime::now_utc()),
        opendesk: "ok",
        database: if database_ok { "ok" } else { "failed" },
        rustdesk: if rustdesk_ok { "ok" } else { "degraded" },
        checks: health_checks
            .into_iter()
            .map(|check| DiagnosticCheck {
                name: check.label,
                status: check.status,
            })
            .collect(),
        backup: DiagnosticBackup {
            status: readiness.status,
            schedule_configured: readiness.schedule_configured,
            destination_configured: readiness.destination_configured,
            execution: readiness.execution,
        },
    }))
}
