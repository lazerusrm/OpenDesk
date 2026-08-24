use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::health::public_key_fingerprint;
use crate::domain::role::Role;
use crate::domain::server_config::{default_server_config, validate_server_config, ServerConfig};
use crate::http::routes::render::render_server_config;
use crate::http::session::{require_action, require_csrf, AuthenticatedUser};
use crate::http::views::{nav_permissions_for_role, NavPermissions};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::server_config::{load_server_config, save_server_config};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings", get(settings_hub))
        .route(
            "/settings/server-config",
            get(server_config_page).post(server_config_submit),
        )
        .route("/settings/key", get(key_page).post(key_submit))
        .route("/settings/relay", get(relay_page).post(relay_submit))
        .route("/settings/others", get(others_page))
}

#[derive(Template)]
#[template(path = "settings_hub.html")]
struct SettingsPageView {
    title: String,
    show_nav: bool,
    nav: NavPermissions,
    csrf_token: String,
    page: String,
    id_server: String,
    relay_server: String,
    api_server: String,
    public_key: String,
    public_key_fingerprint: String,
    message: Option<String>,
    error_message: Option<String>,
}

async fn settings_hub(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigView).await?;
    render_settings_page(
        "hub",
        "Settings",
        &default_server_config(),
        None,
        None,
        &user.csrf_token,
        user.parsed_role(),
    )
}

async fn server_config_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigView).await?;
    let config = load_config(&state).await?;
    Ok(
        render_server_config(&config, None, None, &user.csrf_token, user.parsed_role())
            .into_response(),
    )
}

#[derive(Deserialize)]
struct ServerConfigForm {
    csrf_token: String,
    id_server: String,
    relay_server: String,
    api_server: Option<String>,
    public_key: Option<String>,
}

#[derive(Deserialize)]
struct KeyForm {
    csrf_token: String,
    public_key: Option<String>,
}

#[derive(Deserialize)]
struct RelayForm {
    csrf_token: String,
    id_server: String,
    relay_server: String,
    api_server: Option<String>,
}

async fn server_config_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ServerConfigForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigUpdate).await?;
    require_csrf(&user, &form.csrf_token)?;
    let config = ServerConfig {
        id_server: form.id_server,
        relay_server: form.relay_server,
        api_server: form.api_server.unwrap_or_default(),
        public_key: form.public_key.unwrap_or_default(),
    };
    match persist_config(&state, &user, config).await? {
        Ok(config) => Ok(render_server_config(
            &config,
            Some("Server config saved".to_string()),
            None,
            &user.csrf_token,
            user.parsed_role(),
        )
        .into_response()),
        Err((config, error)) => Ok(render_server_config(
            &config,
            None,
            Some(error),
            &user.csrf_token,
            user.parsed_role(),
        )
        .into_response()),
    }
}

async fn key_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigView).await?;
    let config = load_config(&state).await?;
    render_settings_page(
        "key",
        "Key",
        &config,
        None,
        None,
        &user.csrf_token,
        user.parsed_role(),
    )
}

async fn key_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<KeyForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigUpdate).await?;
    require_csrf(&user, &form.csrf_token)?;
    let mut config = load_config(&state).await?;
    config.public_key = form.public_key.unwrap_or_default();
    render_focused_save(&state, &user, "key", "Key", config).await
}

async fn relay_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigView).await?;
    let config = load_config(&state).await?;
    render_settings_page(
        "relay",
        "Relay",
        &config,
        None,
        None,
        &user.csrf_token,
        user.parsed_role(),
    )
}

async fn relay_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<RelayForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigUpdate).await?;
    require_csrf(&user, &form.csrf_token)?;
    let mut config = load_config(&state).await?;
    config.id_server = form.id_server;
    config.relay_server = form.relay_server;
    config.api_server = form.api_server.unwrap_or_default();
    render_focused_save(&state, &user, "relay", "Relay", config).await
}

async fn others_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::ServerConfigView).await?;
    render_settings_page(
        "others",
        "Others",
        &default_server_config(),
        None,
        None,
        &user.csrf_token,
        user.parsed_role(),
    )
}

async fn load_config(state: &AppState) -> Result<ServerConfig, Response> {
    load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        .map(|config| config.unwrap_or_else(default_server_config))
}

async fn persist_config(
    state: &AppState,
    user: &AuthenticatedUser,
    config: ServerConfig,
) -> Result<Result<ServerConfig, (ServerConfig, String)>, Response> {
    if let Err(error) = validate_server_config(&config) {
        return Ok(Err((config, error.to_string())));
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
    Ok(Ok(config))
}

async fn render_focused_save(
    state: &AppState,
    user: &AuthenticatedUser,
    page: &str,
    title: &str,
    config: ServerConfig,
) -> Result<Response, Response> {
    match persist_config(state, user, config).await? {
        Ok(config) => render_settings_page(
            page,
            title,
            &config,
            Some("Server config saved".to_string()),
            None,
            &user.csrf_token,
            user.parsed_role(),
        ),
        Err((config, error)) => render_settings_page(
            page,
            title,
            &config,
            None,
            Some(error),
            &user.csrf_token,
            user.parsed_role(),
        ),
    }
}

fn render_settings_page(
    page: &str,
    title: &str,
    config: &ServerConfig,
    message: Option<String>,
    error_message: Option<String>,
    csrf_token: &str,
    role: Role,
) -> Result<Response, Response> {
    let view = SettingsPageView {
        title: title.to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(role),
        csrf_token: csrf_token.to_string(),
        page: page.to_string(),
        id_server: config.id_server.clone(),
        relay_server: config.relay_server.clone(),
        api_server: config.api_server.clone(),
        public_key: config.public_key.clone(),
        public_key_fingerprint: public_key_fingerprint(&config.public_key),
        message,
        error_message,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}
