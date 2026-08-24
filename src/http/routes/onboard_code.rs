use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use super::{
    html_form_page, resolve_active_token, unlocked_page, write_onboard_audit, ONBOARD_COOKIE,
    ONBOARD_UNLOCK_SECS,
};
use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::enrollment_token::is_onboard_token_value;
use crate::domain::onboard_totp::{parse_onboard_code, verify_totp};
use crate::http::onboard_views::OnboardView;
use crate::http::session::{constant_time_equal, require_same_origin};
use crate::http::views::NavPermissions;
use crate::login_throttle::request_ip;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::enrollment_tokens::create_enrollment_token;
use crate::repository::onboard_totp::list_active_onboard_totp;
use rand::RngCore;

pub fn routes() -> Router<AppState> {
    Router::new().route("/onboard", get(onboard_index).post(onboard_unlock))
}

const ONBOARD_CSRF_COOKIE: &str = "opendesk_onboard_csrf";

fn code_form(error_message: Option<String>, technician: String, csrf_token: String) -> OnboardView {
    OnboardView {
        title: "Install remote support".to_string(),
        show_nav: false,
        nav: NavPermissions::NONE,
        csrf_token,
        valid: false,
        configured: false,
        show_code_form: true,
        heading: "Enter the technician code".to_string(),
        message: String::new(),
        error_message,
        technician,
        windows_setup_href: String::new(),
        windows_setup_available: false,
        windows_command: String::new(),
        linux_command: String::new(),
        macos_command: String::new(),
    }
}

async fn onboard_index(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    if let Some(token) = jar
        .get(ONBOARD_COOKIE)
        .map(|cookie| cookie.value().to_string())
    {
        if is_onboard_token_value(&token) {
            if let Some(record) = resolve_active_token(&state, &token).await? {
                return unlocked_page(&state, &token, &record).await;
            }
        }
    }
    let (jar, csrf) = ensure_csrf(jar, state.cookie_secure);
    Ok((jar, html_form_page(code_form(None, String::new(), csrf))?).into_response())
}

fn ensure_csrf(jar: CookieJar, secure: bool) -> (CookieJar, String) {
    if let Some(value) = jar
        .get(ONBOARD_CSRF_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .filter(|value| !value.is_empty())
    {
        return (jar, value);
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let value = hex::encode(bytes);
    let mut cookie = Cookie::new(ONBOARD_CSRF_COOKIE, value.clone());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/onboard");
    cookie.set_secure(secure);
    (jar.add(cookie), value)
}

fn csrf_ok(jar: &CookieJar, provided: &str) -> bool {
    jar.get(ONBOARD_CSRF_COOKIE)
        .is_some_and(|cookie| constant_time_equal(cookie.value().as_bytes(), provided.as_bytes()))
}

#[derive(Deserialize)]
struct OnboardCodeForm {
    code: String,
    #[serde(default)]
    technician: String,
    #[serde(default)]
    csrf_token: String,
}

async fn onboard_unlock(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<OnboardCodeForm>,
) -> Result<Response, Response> {
    let (jar, csrf) = ensure_csrf(jar, state.cookie_secure);
    if !require_same_origin(&headers, &state.public_base_url()) || !csrf_ok(&jar, &form.csrf_token)
    {
        return Ok((
            jar,
            html_form_page(code_form(
                Some("Try again from this page.".to_string()),
                form.technician,
                csrf,
            ))?,
        )
            .into_response());
    }
    let ip = request_ip(&headers);
    let now = OffsetDateTime::now_utc();
    if state
        .onboard_guard
        .lock()
        .map(|mut guard| guard.is_blocked(&ip, now))
        .unwrap_or(false)
    {
        return Ok((
            jar,
            html_form_page(code_form(
                Some("Too many attempts. Try again later.".to_string()),
                form.technician,
                csrf,
            ))?,
        )
            .into_response());
    }
    let Some(code) = parse_onboard_code(&form.code) else {
        record_failure(&state, &ip, now);
        return Ok((
            jar,
            html_form_page(code_form(
                Some("Enter the six-digit code from your technician.".to_string()),
                form.technician,
                csrf,
            ))?,
        )
            .into_response());
    };
    let candidates = list_active_onboard_totp(
        &state.db,
        Some(form.technician.as_str()).filter(|value| !value.trim().is_empty()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let mut matches = Vec::new();
    for record in candidates {
        if let Some(timestep) = verify_totp(&record.secret, &code, now) {
            matches.push((record, timestep));
        }
    }
    if matches.len() != 1 {
        record_failure(&state, &ip, now);
        write_unlock_audit(&state, None, "failure").await;
        return Ok((
            jar,
            html_form_page(code_form(
                Some("That code is not valid. Ask your technician for a new one.".to_string()),
                form.technician,
                csrf,
            ))?,
        )
            .into_response());
    }
    let (issuer, timestep) = matches.remove(0);
    let consumed = state
        .onboard_guard
        .lock()
        .map(|mut guard| guard.consume_timestep(issuer.user_uuid, timestep, now))
        .unwrap_or(false);
    if !consumed {
        record_failure(&state, &ip, now);
        write_unlock_audit(&state, Some(issuer.user_uuid), "failure").await;
        return Ok((
            jar,
            html_form_page(code_form(
                Some("That code was already used. Ask your technician for a new one.".to_string()),
                form.technician,
                csrf,
            ))?,
        )
            .into_response());
    }
    let created = create_enrollment_token(
        &state.db,
        "phone-code",
        None,
        Some(now + Duration::minutes(30)),
        Some(issuer.user_uuid),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if let Ok(mut guard) = state.onboard_guard.lock() {
        guard.record_success(&ip);
    }
    write_onboard_audit(&state, &created.record, "onboard_code_unlock").await;
    write_unlock_audit(&state, Some(issuer.user_uuid), "success").await;
    let mut cookie = Cookie::new(ONBOARD_COOKIE, created.token_value.clone());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/onboard");
    cookie.set_max_age(Duration::seconds(ONBOARD_UNLOCK_SECS));
    cookie.set_secure(state.cookie_secure);
    let jar = jar.add(cookie);
    Ok((jar, Redirect::to("/onboard")).into_response())
}

fn record_failure(state: &AppState, ip: &str, now: OffsetDateTime) {
    if let Ok(mut guard) = state.onboard_guard.lock() {
        guard.record_failure(ip, now);
    }
}

async fn write_unlock_audit(state: &AppState, actor: Option<uuid::Uuid>, outcome: &str) {
    let audit = AuditEventDraft {
        actor_user_uuid: actor,
        action: "onboard_code_unlock".to_string(),
        object_type: "enrollment_token".to_string(),
        object_uuid: None,
        outcome: outcome.to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}
