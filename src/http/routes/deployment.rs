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
use uuid::Uuid;

use crate::app_state::AppState;
use crate::deployment::linux_script::{render_linux_deployment_script, LinuxDeploymentScriptInput};
use crate::deployment::windows_script::{
    render_windows_deployment_script, WindowsDeploymentScriptInput,
};
use crate::domain::server_config::{default_server_config, ServerConfig};
use crate::http::routes::render::enrollment_token_status;
use crate::http::session::require_user;
use crate::http::views::{DeploymentView, EnrollmentTokenOptionView};
use crate::repository::enrollment_tokens::list_enrollment_tokens;
use crate::repository::server_config::load_server_config;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/deployment", get(deployment_page))
        .route("/deployment/linux.sh", get(linux_script_export))
}

#[derive(Deserialize)]
pub struct DeploymentQuery {
    pub enrollment_token_uuid: Option<Uuid>,
    pub enrollment_token_value: Option<String>,
}

pub fn enrollment_token_for_script(value: Option<String>) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "PASTE_ENROLLMENT_TOKEN_VALUE".to_string())
}

pub fn render_linux_script_for_deployment(
    config: &ServerConfig,
    enrollment_token_value: Option<String>,
    public_base_url: &str,
) -> String {
    let script_token = enrollment_token_for_script(enrollment_token_value);
    render_linux_deployment_script(&LinuxDeploymentScriptInput {
        server_config: config,
        enrollment_token: &script_token,
        opendesk_base_url: public_base_url,
    })
}

async fn deployment_page(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<DeploymentQuery>,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    let tokens = list_enrollment_tokens(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let selected_uuid = query
        .enrollment_token_uuid
        .or_else(|| tokens.first().map(|token| token.enrollment_token_uuid));
    let token_value = query.enrollment_token_value.unwrap_or_default();
    let token_options: Vec<EnrollmentTokenOptionView> = tokens
        .iter()
        .map(|token| EnrollmentTokenOptionView {
            enrollment_token_uuid: token.enrollment_token_uuid.to_string(),
            label: token.label.clone(),
            status: enrollment_token_status(token),
            selected: Some(token.enrollment_token_uuid) == selected_uuid,
        })
        .collect();
    let linux_script = render_linux_script_for_deployment(
        &config,
        Some(token_value.clone()),
        &state.public_base_url,
    );
    let script_token = enrollment_token_for_script(Some(token_value.clone()));
    let windows_script = render_windows_deployment_script(&WindowsDeploymentScriptInput {
        server_config: &config,
        enrollment_token: &script_token,
        opendesk_base_url: &state.public_base_url,
    });
    let view = DeploymentView {
        title: "Deployment".to_string(),
        show_nav: true,
        tokens: token_options,
        enrollment_token_value: token_value,
        public_base_url: state.public_base_url.clone(),
        linux_script,
        windows_script,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn linux_script_export(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<DeploymentQuery>,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    let script = render_linux_script_for_deployment(
        &config,
        query.enrollment_token_value,
        &state.public_base_url,
    );
    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        script,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrollment_token_for_script_uses_provided_value() {
        assert_eq!(
            enrollment_token_for_script(Some("abc123".to_string())),
            "abc123"
        );
    }

    #[test]
    fn enrollment_token_for_script_falls_back_to_placeholder() {
        assert_eq!(
            enrollment_token_for_script(Some("  ".to_string())),
            "PASTE_ENROLLMENT_TOKEN_VALUE"
        );
    }
}