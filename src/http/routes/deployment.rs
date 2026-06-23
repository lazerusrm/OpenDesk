use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
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
use crate::domain::server_config::default_server_config;
use crate::http::routes::render::enrollment_token_status;
use crate::http::session::require_user;
use crate::http::views::{DeploymentView, EnrollmentTokenOptionView};
use crate::repository::enrollment_tokens::list_enrollment_tokens;
use crate::repository::server_config::load_server_config;

pub fn routes() -> Router<AppState> {
    Router::new().route("/deployment", get(deployment_page))
}

#[derive(Deserialize)]
struct DeploymentQuery {
    enrollment_token_uuid: Option<Uuid>,
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
    let token_options: Vec<EnrollmentTokenOptionView> = tokens
        .iter()
        .map(|token| EnrollmentTokenOptionView {
            enrollment_token_uuid: token.enrollment_token_uuid.to_string(),
            label: token.label.clone(),
            status: enrollment_token_status(token),
            selected: Some(token.enrollment_token_uuid) == selected_uuid,
        })
        .collect();
    let placeholder_token = "REPLACE_WITH_ENROLLMENT_TOKEN";
    let linux_script = render_linux_deployment_script(&LinuxDeploymentScriptInput {
        server_config: &config,
        enrollment_token: placeholder_token,
        opendesk_base_url: "https://rd-admin.example.com",
    });
    let windows_script = render_windows_deployment_script(&WindowsDeploymentScriptInput {
        server_config: &config,
        enrollment_token: placeholder_token,
        opendesk_base_url: "https://rd-admin.example.com",
    });
    let view = DeploymentView {
        title: "Deployment".to_string(),
        show_nav: true,
        tokens: token_options,
        linux_script,
        windows_script,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}