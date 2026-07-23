use askama::Template;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};

use crate::app_state::AppState;
use crate::deployment::filename_config::generate_filename_custom_server;
use crate::deployment::linux_script::{render_linux_deployment_script, LinuxDeploymentScriptInput};
use crate::deployment::macos_script::{render_macos_deployment_script, MacosDeploymentScriptInput};
use crate::deployment::windows_script::{
    render_windows_deployment_script, WindowsDeploymentScriptInput,
};
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::server_config::ServerConfig;
use crate::http::session::{require_action, AuthenticatedUser};
use crate::http::views::{nav_permissions_for_role, DeploymentDownloadView, DeploymentView};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::server_config::load_server_config;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/deployment", get(deployment_page))
        .route("/deployment/linux.sh", get(linux_script_export))
}

pub fn rustdesk_config_payload(config: &ServerConfig) -> String {
    let json = serde_json::json!({
        "host": config.id_server.trim(),
        "relay": config.relay_server.trim(),
        "api": config.api_server.trim(),
        "key": config.public_key.trim(),
    })
    .to_string();
    let encoded = URL_SAFE.encode(json.as_bytes());
    format!("config={}", encoded.chars().rev().collect::<String>())
}

fn rustdesk_qr_svg(payload: &str) -> Result<String, qrcode::types::QrError> {
    Ok(qrcode::QrCode::new(payload)?
        .render::<qrcode::render::svg::Color>()
        .build())
}

fn deployment_downloads(state: &AppState) -> Vec<DeploymentDownloadView> {
    const RELEASES: &str = "https://github.com/rustdesk/rustdesk/releases";
    [
        (
            "Windows",
            "Official desktop packages",
            state.rustdesk_download_windows_url.as_deref(),
        ),
        (
            "macOS",
            "Official Apple packages",
            state.rustdesk_download_macos_url.as_deref(),
        ),
        (
            "Linux",
            "Official distribution packages",
            state.rustdesk_download_linux_url.as_deref(),
        ),
        (
            "Android",
            "Official mobile packages",
            state.rustdesk_download_android_url.as_deref(),
        ),
    ]
    .into_iter()
    .map(
        |(platform, description, configured_url)| DeploymentDownloadView {
            platform: platform.to_string(),
            description: description.to_string(),
            url: configured_url.unwrap_or(RELEASES).to_string(),
        },
    )
    .collect()
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
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeploymentView).await?;
    let stored_config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let configured = stored_config.is_some();
    let config = stored_config.unwrap_or(ServerConfig {
        id_server: String::new(),
        relay_server: String::new(),
        api_server: String::new(),
        public_key: String::new(),
    });
    let linux_script = render_linux_script_for_deployment(&config, None, &state.public_base_url);
    let script_token = enrollment_token_for_script(None);
    let windows_script = render_windows_deployment_script(&WindowsDeploymentScriptInput {
        server_config: &config,
        enrollment_token: &script_token,
        opendesk_base_url: &state.public_base_url,
    });
    let macos_script = render_macos_deployment_script(&MacosDeploymentScriptInput {
        server_config: &config,
        enrollment_token: &script_token,
        opendesk_base_url: &state.public_base_url,
    });
    let filename_custom_server = generate_filename_custom_server(&config);
    write_deployment_audit(
        &state,
        &user,
        "deployment_artifact_generate",
        serde_json::json!({
            "artifacts": ["linux", "windows", "macos", "filename_custom_server"],
        }),
    )
    .await;
    let rustdesk_config = if configured {
        rustdesk_config_payload(&config)
    } else {
        String::new()
    };
    let rustdesk_qr_svg = if configured {
        rustdesk_qr_svg(&rustdesk_config)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    } else {
        String::new()
    };
    let view = DeploymentView {
        title: "Deployment Center".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        configured,
        id_server: config.id_server.clone(),
        relay_server: config.relay_server.clone(),
        api_server: config.api_server.clone(),
        public_key: config.public_key.clone(),
        rustdesk_config,
        rustdesk_qr_svg,
        downloads: deployment_downloads(&state),
        public_base_url: state.public_base_url.clone(),
        linux_script,
        windows_script,
        macos_script,
        filename_custom_server,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn linux_script_export(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeploymentScriptExport).await?;
    let config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::CONFLICT.into_response())?;
    let script = render_linux_script_for_deployment(&config, None, &state.public_base_url);
    write_deployment_audit(
        &state,
        &user,
        "deployment_script_export",
        serde_json::json!({
            "artifact": "linux.sh",
            "enrollment_token_provided": false,
        }),
    )
    .await;
    Ok((
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        script,
    )
        .into_response())
}

async fn write_deployment_audit(
    state: &AppState,
    user: &AuthenticatedUser,
    action: &str,
    detail: serde_json::Value,
) {
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: action.to_string(),
        object_type: "deployment_artifact".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: Some(detail),
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustdesk_config_payload_uses_official_client_shape_without_secrets() {
        let config = ServerConfig {
            id_server: "id.example.com".to_string(),
            relay_server: "relay.example.com".to_string(),
            api_server: "https://console.example.com".to_string(),
            public_key: "public-key".to_string(),
        };
        let payload = rustdesk_config_payload(&config);
        assert!(payload.starts_with("config="));
        let reversed: String = payload[7..].chars().rev().collect();
        let decoded = URL_SAFE.decode(reversed).expect("base64");
        let value: serde_json::Value = serde_json::from_slice(&decoded).expect("json");
        assert_eq!(value["host"], "id.example.com");
        assert_eq!(value["relay"], "relay.example.com");
        assert_eq!(value["api"], "https://console.example.com");
        assert_eq!(value["key"], "public-key");
        assert_eq!(value.as_object().expect("object").len(), 4);
    }

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
