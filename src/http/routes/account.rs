use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::auth::password_meets_policy;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::onboard_totp::encode_base32;
use crate::http::session::{end_session, require_csrf, require_user, AuthenticatedUser};
use crate::http::views::{nav_permissions_for_role, AccountView};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::onboard_totp::user_has_onboard_totp;
use crate::repository::users::{authenticate_user_password, reset_user_password};

#[path = "account_onboard.rs"]
mod account_onboard;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/account", get(account_page))
        .route("/account/password", post(account_password_submit))
        .route("/onboard-setup", get(account_onboard::onboard_setup_page))
        .route(
            "/onboard-setup/start",
            post(account_onboard::start_onboard_totp),
        )
        .route(
            "/onboard-setup/confirm",
            post(account_onboard::confirm_onboard_totp),
        )
        .route(
            "/onboard-setup/revoke",
            post(account_onboard::revoke_onboard_totp),
        )
}

async fn account_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    Ok(render_account_page(&state, &user, None, None, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response())
}

#[derive(Deserialize)]
struct PasswordChangeForm {
    csrf_token: String,
    current_password: String,
    new_password: String,
    confirm_password: String,
}

async fn account_password_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<PasswordChangeForm>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    require_csrf(&user, &form.csrf_token)?;
    let current = authenticate_user_password(&state.db, &user.username, &form.current_password)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if current.is_none() {
        return Ok(render_account_page(
            &state,
            &user,
            Some("current password is incorrect".to_string()),
            None,
            None,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    if form.new_password != form.confirm_password {
        return Ok(render_account_page(
            &state,
            &user,
            Some("new password and confirmation do not match".to_string()),
            None,
            None,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    if !password_meets_policy(&form.new_password) {
        return Ok(render_account_page(
            &state,
            &user,
            Some("password must be at least 8 characters".to_string()),
            None,
            None,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    reset_user_password(
        &state.db,
        &user.username,
        form.new_password.trim(),
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "account_password_change".to_string(),
        object_type: "user".to_string(),
        object_uuid: Some(user.user_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    let jar = end_session(&state, jar).await;
    Ok((jar, Redirect::to("/login?login=password-updated")).into_response())
}

pub(super) async fn render_account_page(
    state: &AppState,
    user: &AuthenticatedUser,
    error_message: Option<String>,
    onboard_error: Option<String>,
    onboard_notice: Option<String>,
) -> Result<Html<String>, sqlx::Error> {
    let can_manage_onboard_totp = Action::EnrollmentTokenCreate.allowed_for(user.parsed_role());
    let onboard_totp_enrolled = if can_manage_onboard_totp {
        user_has_onboard_totp(&state.db, user.user_uuid).await?
    } else {
        false
    };
    let pending = account_onboard::pending_secret(state, user.user_uuid);
    let (onboard_totp_qr_svg, onboard_totp_secret) = pending
        .as_deref()
        .map(|secret| {
            (
                account_onboard::qr_svg(&user.username, secret).unwrap_or_default(),
                encode_base32(secret),
            )
        })
        .unwrap_or_default();
    let view = AccountView {
        title: "Account".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        username: user.username.clone(),
        role_display: user.parsed_role().display_label().to_string(),
        error_message,
        can_manage_onboard_totp,
        onboard_totp_enrolled,
        onboard_totp_qr_svg,
        onboard_totp_secret,
        onboard_notice,
        onboard_error,
    };
    Ok(Html(view.render().expect("render account")))
}
