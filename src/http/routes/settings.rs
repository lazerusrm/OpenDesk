use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::server_config::{default_server_config, validate_server_config, ServerConfig};
use crate::http::routes::render::render_server_config;
use crate::http::session::require_user;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::server_config::{load_server_config, save_server_config};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/settings/server-config",
        get(server_config_page).post(server_config_submit),
    )
}

async fn server_config_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    Ok(render_server_config(&config, None, None).into_response())
}

#[derive(Deserialize)]
struct ServerConfigForm {
    id_server: String,
    relay_server: String,
    api_server: Option<String>,
    public_key: Option<String>,
}

async fn server_config_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ServerConfigForm>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let config = ServerConfig {
        id_server: form.id_server,
        relay_server: form.relay_server,
        api_server: form.api_server.unwrap_or_default(),
        public_key: form.public_key.unwrap_or_default(),
    };
    if let Err(error) = validate_server_config(&config) {
        return Ok(render_server_config(&config, None, Some(error.to_string())).into_response());
    }
    save_server_config(&state.db, &config, Some(user.user_uuid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "server_config_update".to_string(),
        object_type: "server_config".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(render_server_config(&config, Some("Server config saved".to_string()), None).into_response())
}