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
use crate::http::deployment_views::{DeploymentDownloadView, DeploymentOsView, DeploymentView};
use crate::http::session::{require_action, AuthenticatedUser};
use crate::http::views::nav_permissions_for_role;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::server_config::load_server_config;

const SCRIPT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

#[path = "deployment_artifacts.rs"]
mod deployment_artifacts;
#[path = "deployment_os.rs"]
mod deployment_os;
use deployment_os::{DeploymentOs, RUSTDESK_RELEASES_URL};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/deployment", get(deployment_page))
        .route("/deployment/windows", get(windows_page))
        .route("/deployment/linux", get(linux_page))
        .route("/deployment/macos", get(macos_page))
        .route("/deployment/android", get(android_page))
        .route("/deployment/ios", get(ios_page))
        .route("/deployment/linux.sh", get(linux_script_export))
        .route("/deployment/windows.ps1", get(windows_script_export))
        .route("/deployment/macos.sh", get(macos_script_export))
        .route(
            "/deployment/windows/{name}",
            get(deployment_artifacts::windows_setup_file),
        )
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

fn deployment_os_cards() -> Vec<DeploymentDownloadView> {
    [
        DeploymentOs::Windows,
        DeploymentOs::Linux,
        DeploymentOs::Macos,
        DeploymentOs::Android,
        DeploymentOs::Ios,
    ]
    .into_iter()
    .map(|os| DeploymentDownloadView {
        platform: os.platform().to_string(),
        description: os.description().to_string(),
        page_href: os.page_href().to_string(),
    })
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

fn render_os_script(os: DeploymentOs, config: &ServerConfig, public_base_url: &str) -> String {
    let script_token = enrollment_token_for_script(None);
    match os {
        DeploymentOs::Windows => render_windows_deployment_script(&WindowsDeploymentScriptInput {
            server_config: config,
            enrollment_token: &script_token,
            opendesk_base_url: public_base_url,
        }),
        DeploymentOs::Linux => render_linux_script_for_deployment(config, None, public_base_url),
        DeploymentOs::Macos => render_macos_deployment_script(&MacosDeploymentScriptInput {
            server_config: config,
            enrollment_token: &script_token,
            opendesk_base_url: public_base_url,
        }),
        DeploymentOs::Android | DeploymentOs::Ios => String::new(),
    }
}

fn empty_config() -> ServerConfig {
    ServerConfig {
        id_server: String::new(),
        relay_server: String::new(),
        api_server: String::new(),
        public_key: String::new(),
    }
}

async fn load_optional_config(state: &AppState) -> Result<Option<ServerConfig>, Response> {
    load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn config_payload_and_qr(
    config: &ServerConfig,
    configured: bool,
) -> Result<(String, String), Response> {
    if !configured {
        return Ok((String::new(), String::new()));
    }
    let rustdesk_config = rustdesk_config_payload(config);
    let rustdesk_qr_svg = rustdesk_qr_svg(&rustdesk_config)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok((rustdesk_config, rustdesk_qr_svg))
}

async fn deployment_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeploymentView).await?;
    let stored_config = load_optional_config(&state).await?;
    let configured = stored_config.is_some();
    let config = stored_config.unwrap_or_else(empty_config);
    let (rustdesk_config, rustdesk_qr_svg) = config_payload_and_qr(&config, configured)?;
    let view = DeploymentView {
        title: "Deployment Center".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        configured,
        id_server: config.id_server,
        relay_server: config.relay_server,
        api_server: config.api_server,
        public_key: config.public_key,
        rustdesk_config,
        rustdesk_qr_svg,
        downloads: deployment_os_cards(),
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn windows_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    deployment_os_page(state, jar, DeploymentOs::Windows).await
}

async fn linux_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    deployment_os_page(state, jar, DeploymentOs::Linux).await
}

async fn macos_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    deployment_os_page(state, jar, DeploymentOs::Macos).await
}

async fn android_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    deployment_os_page(state, jar, DeploymentOs::Android).await
}

async fn ios_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    deployment_os_page(state, jar, DeploymentOs::Ios).await
}

async fn deployment_os_page(
    state: AppState,
    jar: CookieJar,
    os: DeploymentOs,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeploymentView).await?;
    let stored_config = load_optional_config(&state).await?;
    let configured = stored_config.is_some();
    let config = stored_config.unwrap_or_else(empty_config);
    let (rustdesk_config, rustdesk_qr_svg) = config_payload_and_qr(&config, configured)?;
    let script_export = os.script_export();
    let show_script = script_export.is_some();
    let show_filename_fallback = matches!(os, DeploymentOs::Windows);
    let script = if configured && show_script {
        render_os_script(os, &config, &state.public_base_url)
    } else {
        String::new()
    };
    let filename_custom_server = if configured && show_filename_fallback {
        generate_filename_custom_server(&config)
    } else {
        String::new()
    };
    if configured {
        let mut artifacts = Vec::new();
        if show_script {
            artifacts.push(os.platform().to_lowercase());
        }
        if show_filename_fallback {
            artifacts.push("filename_custom_server".to_string());
        }
        if !artifacts.is_empty() {
            write_deployment_audit(
                &state,
                &user,
                "deployment_artifact_generate",
                serde_json::json!({ "artifacts": artifacts }),
            )
            .await;
        }
    }
    let (script_download_href, script_download_label, script_heading) = script_export
        .map(|(href, label, heading)| (href.to_string(), label.to_string(), heading.to_string()))
        .unwrap_or_default();
    let view = DeploymentOsView {
        title: format!("{} deployment", os.platform()),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        configured,
        platform: os.platform().to_string(),
        description: os.description().to_string(),
        official_download_url: os.official_download_url(&state).to_string(),
        id_server: config.id_server,
        relay_server: config.relay_server,
        api_server: config.api_server,
        public_key: config.public_key,
        rustdesk_config,
        rustdesk_qr_svg,
        public_base_url: state.public_base_url.clone(),
        show_script,
        script_heading,
        script,
        script_download_href,
        script_download_label,
        show_filename_fallback,
        filename_custom_server,
        is_android: matches!(os, DeploymentOs::Android),
        is_ios: matches!(os, DeploymentOs::Ios),
        first_party_windows_downloads: if matches!(os, DeploymentOs::Windows) {
            deployment_artifacts::windows_downloads(&state)
        } else {
            Vec::new()
        },
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
    export_deployment_script(state, jar, "linux.sh", DeploymentOs::Linux).await
}

async fn windows_script_export(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    export_deployment_script(state, jar, "windows.ps1", DeploymentOs::Windows).await
}

async fn macos_script_export(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    export_deployment_script(state, jar, "macos.sh", DeploymentOs::Macos).await
}

async fn export_deployment_script(
    state: AppState,
    jar: CookieJar,
    artifact: &str,
    os: DeploymentOs,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeploymentScriptExport).await?;
    let config = load_optional_config(&state)
        .await?
        .ok_or_else(|| StatusCode::CONFLICT.into_response())?;
    let script = render_os_script(os, &config, &state.public_base_url);
    write_deployment_audit(
        &state,
        &user,
        "deployment_script_export",
        serde_json::json!({
            "artifact": artifact,
            "enrollment_token_provided": false,
        }),
    )
    .await;
    Ok(([(header::CONTENT_TYPE, SCRIPT_CONTENT_TYPE)], script).into_response())
}

pub(super) async fn write_deployment_audit(
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
