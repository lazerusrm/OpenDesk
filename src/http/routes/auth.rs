use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::http::routes::render::render_login;
use crate::http::session::{end_session, require_csrf, require_present_same_origin, start_session};
use crate::login_throttle::request_ip;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::users::authenticate_user_password;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout))
}

#[derive(Deserialize)]
struct LoginQuery {
    login: Option<String>,
    notice: Option<String>,
}

async fn login_page(Query(query): Query<LoginQuery>) -> impl IntoResponse {
    let notice_key = query.notice.as_deref().or(query.login.as_deref());
    let notice_message = match notice_key {
        Some("password-updated") => {
            Some("Password updated. Sign in with your new password.".to_string())
        }
        _ => None,
    };
    render_login(None, notice_message)
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LogoutForm {
    csrf_token: String,
}

async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<LoginForm>,
) -> Result<Response, StatusCode> {
    if !require_present_same_origin(&headers, &state.public_base_url) {
        return Err(StatusCode::FORBIDDEN);
    }
    let username = form.username.trim();
    let ip = request_ip(&headers);
    let now = OffsetDateTime::now_utc();
    if form.password.len() > 1024 {
        return Ok(
            render_login(Some("Invalid username or password".to_string()), None).into_response(),
        );
    }
    if state
        .login_throttle
        .lock()
        .map(|mut throttle| throttle.is_blocked(username, &ip, now))
        .unwrap_or(false)
    {
        return Ok(render_login(
            Some("Too many sign-in attempts. Try again later.".to_string()),
            None,
        )
        .into_response());
    }
    let user = authenticate_user_password(&state.db, username, &form.password)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(user) = user else {
        if let Ok(mut throttle) = state.login_throttle.lock() {
            throttle.record_failure(username, &ip, now);
        }
        let audit = AuditEventDraft {
            actor_user_uuid: None,
            action: "login".to_string(),
            object_type: "session".to_string(),
            object_uuid: None,
            outcome: "failure".to_string(),
            source: "web".to_string(),
            detail: None,
        };
        let _ = insert_audit_event(&state.db, &audit).await;
        return Ok(
            render_login(Some("Invalid username or password".to_string()), None).into_response(),
        );
    };
    if let Ok(mut throttle) = state.login_throttle.lock() {
        throttle.record_success(username, &ip);
    }
    let (jar, _) = start_session(&state, jar, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "login".to_string(),
        object_type: "session".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok((jar, Redirect::to("/")).into_response())
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LogoutForm>,
) -> Result<impl IntoResponse, Response> {
    let user = crate::http::session::require_user(&state, &jar).await?;
    require_csrf(&user, &form.csrf_token)?;
    let jar = end_session(&state, jar).await;
    Ok((jar, Redirect::to("/login")))
}
