use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Form,
};
use axum_extra::extract::cookie::CookieJar;
use qrcode::render::svg;
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::onboard_totp::{encode_base32, generate_totp_secret, otpauth_url, verify_totp};
use crate::http::onboard_views::OnboardSetupView;
use crate::http::session::{require_action, require_csrf, AuthenticatedUser};
use crate::http::views::nav_permissions_for_role;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::onboard_totp::{
    delete_user_onboard_totp, save_user_onboard_totp, user_has_onboard_totp,
};
use askama::Template;
use axum::response::Html;

#[derive(Deserialize)]
pub struct EmptyCsrfForm {
    pub csrf_token: String,
}

#[derive(Deserialize)]
pub struct ConfirmTotpForm {
    pub csrf_token: String,
    pub code: String,
}

pub async fn start_onboard_totp(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<EmptyCsrfForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenCreate).await?;
    require_csrf(&user, &form.csrf_token)?;
    if user_has_onboard_totp(&state.db, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    {
        return Ok(setup_page(
            &state,
            &user,
            Some("revoke the current authenticator first".to_string()),
            None,
        )
        .await?);
    }
    let secret = generate_totp_secret();
    if let Ok(mut pending) = state.pending_onboard_totp.lock() {
        pending.insert(user.user_uuid, secret);
    }
    setup_page(
        &state,
        &user,
        None,
        Some("scan the code, then enter a six-digit value to confirm".to_string()),
    )
    .await
}

pub async fn confirm_onboard_totp(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<ConfirmTotpForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenCreate).await?;
    require_csrf(&user, &form.csrf_token)?;
    let secret = state
        .pending_onboard_totp
        .lock()
        .ok()
        .and_then(|pending| pending.get(&user.user_uuid).cloned());
    let Some(secret) = secret else {
        return Ok(setup_page(
            &state,
            &user,
            Some("start authenticator setup first".to_string()),
            None,
        )
        .await?);
    };
    if verify_totp(&secret, &form.code, time::OffsetDateTime::now_utc()).is_none() {
        return Ok(setup_page(
            &state,
            &user,
            Some("authenticator code is not valid".to_string()),
            None,
        )
        .await?);
    }
    save_user_onboard_totp(&state.db, user.user_uuid, &secret)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if let Ok(mut pending) = state.pending_onboard_totp.lock() {
        pending.remove(&user.user_uuid);
    }
    write_totp_audit(&state, user.user_uuid, "onboard_totp_enroll").await;
    setup_page(
        &state,
        &user,
        None,
        Some(
            "authenticator enrolled. Recipients can use /onboard with your current code."
                .to_string(),
        ),
    )
    .await
}

pub async fn revoke_onboard_totp(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<EmptyCsrfForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenCreate).await?;
    require_csrf(&user, &form.csrf_token)?;
    delete_user_onboard_totp(&state.db, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if let Ok(mut pending) = state.pending_onboard_totp.lock() {
        pending.remove(&user.user_uuid);
    }
    write_totp_audit(&state, user.user_uuid, "onboard_totp_revoke").await;
    setup_page(
        &state,
        &user,
        None,
        Some("authenticator revoked".to_string()),
    )
    .await
}

pub async fn onboard_setup_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenCreate).await?;
    setup_page(&state, &user, None, None).await
}

async fn setup_page(
    state: &AppState,
    user: &AuthenticatedUser,
    onboard_error: Option<String>,
    onboard_notice: Option<String>,
) -> Result<Response, Response> {
    let onboard_totp_enrolled = user_has_onboard_totp(&state.db, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let pending = pending_secret(state, user.user_uuid);
    let (onboard_totp_qr_svg, onboard_totp_secret) = pending
        .as_deref()
        .map(|secret| {
            (
                qr_svg(&user.username, secret).unwrap_or_default(),
                encode_base32(secret),
            )
        })
        .unwrap_or_default();
    let view = OnboardSetupView {
        title: "Onboard codes".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        username: user.username.clone(),
        onboard_totp_enrolled,
        onboard_totp_qr_svg,
        onboard_totp_secret,
        onboard_notice,
        onboard_error,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

pub fn pending_secret(state: &AppState, user_uuid: Uuid) -> Option<Vec<u8>> {
    state
        .pending_onboard_totp
        .lock()
        .ok()
        .and_then(|pending| pending.get(&user_uuid).cloned())
}

pub fn qr_svg(username: &str, secret: &[u8]) -> Result<String, qrcode::types::QrError> {
    Ok(qrcode::QrCode::new(otpauth_url(username, secret))?
        .render::<svg::Color>()
        .build())
}

async fn write_totp_audit(state: &AppState, user_uuid: Uuid, action: &str) {
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user_uuid),
        action: action.to_string(),
        object_type: "user".to_string(),
        object_uuid: Some(user_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}
