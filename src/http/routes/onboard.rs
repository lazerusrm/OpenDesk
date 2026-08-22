use askama::Template;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::deployment::onboard_linux::{render_onboard_linux_script, OnboardLinuxScriptInput};
use crate::deployment::onboard_macos::{render_onboard_macos_script, OnboardMacosScriptInput};
use crate::deployment::onboard_overlay::stamp_windows_setup;
use crate::deployment::onboard_windows::{
    render_onboard_windows_script, OnboardWindowsScriptInput,
};
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::enrollment_token::{
    hash_enrollment_token_value, is_onboard_token_value, onboard_url,
    verify_enrollment_token_value, EnrollmentTokenRecord,
};
use crate::domain::server_config::ServerConfig;
use crate::http::onboard_views::OnboardView;
use crate::http::views::NavPermissions;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::enrollment_tokens::find_enrollment_token_by_hash;
use crate::repository::server_config::load_server_config;

const SCRIPT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const SETUP_CONTENT_TYPE: &str = "application/vnd.microsoft.portable-executable";
const INVALID_HEADING: &str = "Install link unavailable";
const INVALID_MESSAGE: &str =
    "This install link is invalid or has expired. Ask your technician for a new link.";
pub(super) const ONBOARD_COOKIE: &str = "opendesk_onboard";
pub(super) const ONBOARD_UNLOCK_SECS: i64 = 30 * 60;

#[path = "onboard_code.rs"]
mod onboard_code;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(onboard_code::routes())
        .route("/onboard/{token}", get(onboard_page))
        .route("/onboard/{token}/linux.sh", get(linux_script))
        .route("/onboard/{token}/windows.ps1", get(windows_script))
        .route("/onboard/{token}/macos.sh", get(macos_script))
        .route("/onboard/{token}/windows/{name}", get(windows_setup_file))
}

pub(super) fn apply_public_headers(mut response: Response) -> Response {
    apply_cache_headers(&mut response, "no-referrer");
    response
}

pub(super) fn apply_form_headers(mut response: Response) -> Response {
    apply_cache_headers(&mut response, "same-origin");
    response
}

fn apply_cache_headers(response: &mut Response, referrer_policy: &'static str) {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static(referrer_policy),
    );
    headers.insert(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex"),
    );
}

pub(super) fn html_page(view: OnboardView) -> Result<Response, Response> {
    html_page_referrer(view, true)
}

pub(super) fn html_form_page(view: OnboardView) -> Result<Response, Response> {
    html_page_referrer(view, false)
}

fn html_page_referrer(view: OnboardView, hide_referrer: bool) -> Result<Response, Response> {
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let response = Html(html).into_response();
    if hide_referrer {
        Ok(apply_public_headers(response))
    } else {
        Ok(apply_form_headers(response))
    }
}

fn invalid_view() -> OnboardView {
    OnboardView {
        title: "Install remote support".to_string(),
        show_nav: false,
        nav: NavPermissions::NONE,
        csrf_token: String::new(),
        valid: false,
        configured: false,
        show_code_form: false,
        heading: INVALID_HEADING.to_string(),
        message: INVALID_MESSAGE.to_string(),
        error_message: None,
        technician: String::new(),
        windows_setup_href: String::new(),
        windows_setup_available: false,
        windows_command: String::new(),
        linux_command: String::new(),
        macos_command: String::new(),
    }
}

fn copy_command(script_url: &str, kind: CopyKind) -> String {
    match kind {
        CopyKind::Linux => format!(
            "curl -fsSL '{}' | sudo bash",
            script_url.replace('\'', "'\\''")
        ),
        CopyKind::Macos => format!("curl -fsSL '{}' | bash", script_url.replace('\'', "'\\''")),
        CopyKind::Windows => format!("irm '{}' | iex", script_url.replace('\'', "''")),
    }
}

enum CopyKind {
    Linux,
    Macos,
    Windows,
}

pub(super) fn signed_windows_setup_path(state: &AppState) -> Option<std::path::PathBuf> {
    let dir = state.signed_client_dir.as_ref()?;
    let path = dir.join("windows-setup.exe");
    path.is_file().then_some(path)
}

pub(super) async fn resolve_active_token(
    state: &AppState,
    token: &str,
) -> Result<Option<EnrollmentTokenRecord>, Response> {
    if !is_onboard_token_value(token) {
        return Ok(None);
    }
    let token_hash = hash_enrollment_token_value(token);
    let Some(record) = find_enrollment_token_by_hash(&state.db, &token_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    else {
        return Ok(None);
    };
    if verify_enrollment_token_value(&record, token, OffsetDateTime::now_utc()).is_err() {
        return Ok(None);
    }
    Ok(Some(record))
}

fn invalid_page() -> Result<Response, Response> {
    let mut response = html_page(invalid_view())?;
    *response.status_mut() = StatusCode::NOT_FOUND;
    Ok(response)
}

async fn require_token(
    state: &AppState,
    token: &str,
    as_page: bool,
) -> Result<EnrollmentTokenRecord, Response> {
    match resolve_active_token(state, token).await? {
        Some(record) => Ok(record),
        None if as_page => Err(invalid_page()?),
        None => Err(StatusCode::NOT_FOUND.into_response()),
    }
}

pub(super) async fn load_config(state: &AppState) -> Result<Option<ServerConfig>, Response> {
    load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub(super) async fn write_onboard_audit(
    state: &AppState,
    record: &EnrollmentTokenRecord,
    action: &str,
) {
    let audit = AuditEventDraft {
        actor_user_uuid: None,
        action: action.to_string(),
        object_type: "enrollment_token".to_string(),
        object_uuid: Some(record.enrollment_token_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}

pub(super) async fn unlocked_page(
    state: &AppState,
    token: &str,
    record: &EnrollmentTokenRecord,
) -> Result<Response, Response> {
    let configured = load_config(state).await?.is_some();
    let base = onboard_url(&state.public_base_url, token);
    let windows_setup_available = signed_windows_setup_path(state).is_some();
    write_onboard_audit(state, record, "onboard_page_view").await;
    html_page(OnboardView {
        title: "Install remote support".to_string(),
        show_nav: false,
        nav: NavPermissions::NONE,
        csrf_token: String::new(),
        valid: true,
        configured,
        show_code_form: false,
        heading: "Install remote support".to_string(),
        message: "Download or paste the command for this computer. Your technician will be able to help after setup finishes.".to_string(),
        error_message: None,
        technician: String::new(),
        windows_setup_href: format!("{base}/windows/setup.exe"),
        windows_setup_available,
        windows_command: copy_command(&format!("{base}/windows.ps1"), CopyKind::Windows),
        linux_command: copy_command(&format!("{base}/linux.sh"), CopyKind::Linux),
        macos_command: copy_command(&format!("{base}/macos.sh"), CopyKind::Macos),
    })
}

async fn onboard_page(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, Response> {
    let record = require_token(&state, &token, true).await?;
    unlocked_page(&state, &token, &record).await
}

async fn linux_script(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, Response> {
    let record = require_token(&state, &token, false).await?;
    let config = load_config(&state)
        .await?
        .ok_or_else(|| StatusCode::CONFLICT.into_response())?;
    let script = render_onboard_linux_script(&OnboardLinuxScriptInput {
        server_config: &config,
        enrollment_token: &token,
        opendesk_base_url: &state.public_base_url,
        linux_package_url: state.rustdesk_download_linux_url.as_deref(),
    });
    write_onboard_audit(&state, &record, "onboard_script_download").await;
    Ok(script_response("opendesk-onboard.sh", script))
}

async fn windows_script(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, Response> {
    let record = require_token(&state, &token, false).await?;
    let config = load_config(&state)
        .await?
        .ok_or_else(|| StatusCode::CONFLICT.into_response())?;
    let setup_url = format!(
        "{}/windows/setup.exe",
        onboard_url(&state.public_base_url, &token)
    );
    let script = render_onboard_windows_script(&OnboardWindowsScriptInput {
        server_config: &config,
        enrollment_token: &token,
        opendesk_base_url: &state.public_base_url,
        setup_url: &setup_url,
    });
    write_onboard_audit(&state, &record, "onboard_script_download").await;
    Ok(script_response("opendesk-onboard.ps1", script))
}

async fn macos_script(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, Response> {
    let record = require_token(&state, &token, false).await?;
    let config = load_config(&state)
        .await?
        .ok_or_else(|| StatusCode::CONFLICT.into_response())?;
    let script = render_onboard_macos_script(&OnboardMacosScriptInput {
        server_config: &config,
        enrollment_token: &token,
        opendesk_base_url: &state.public_base_url,
    });
    write_onboard_audit(&state, &record, "onboard_script_download").await;
    Ok(script_response("opendesk-onboard-macos.sh", script))
}

async fn windows_setup_file(
    State(state): State<AppState>,
    Path((token, name)): Path<(String, String)>,
) -> Result<Response, Response> {
    let record = require_token(&state, &token, false).await?;
    if name != "setup.exe" {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    let path =
        signed_windows_setup_path(&state).ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let checkin_url = format!(
        "{}/api/enrollments/check-in",
        state.public_base_url.trim_end_matches('/')
    );
    let stamped = stamp_windows_setup(&bytes, &token, &checkin_url);
    write_onboard_audit(&state, &record, "onboard_artifact_download").await;
    let mut response = Response::new(Body::from(stamped));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(SETUP_CONTENT_TYPE),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"opendesk-windows-setup.exe\""),
    );
    Ok(apply_public_headers(response))
}

fn script_response(filename: &str, script: String) -> Response {
    let mut response = Response::new(Body::from(script));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(SCRIPT_CONTENT_TYPE),
    );
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    apply_public_headers(response)
}
