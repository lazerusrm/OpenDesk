use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;

use crate::app_state::AppState;
use crate::domain::health::{build_health_checks, public_key_fingerprint};
use crate::domain::server_config::default_server_config;
use crate::http::session::require_user;
use crate::http::views::{HealthCheckRowView, StatusView};
use crate::repository::server_config::load_server_config;

pub fn routes() -> Router<AppState> {
    Router::new().route("/status", get(status_page))
}

async fn status_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
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
    let view = StatusView {
        title: "Status".to_string(),
        show_nav: true,
        id_server: config.id_server.clone(),
        relay_server: config.relay_server.clone(),
        public_key_fingerprint: public_key_fingerprint(&config.public_key),
        checks,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}