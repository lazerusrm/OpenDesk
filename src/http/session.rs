use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::session::session_is_valid;
use crate::repository::sessions::{create_session, delete_session, find_session};
use crate::repository::users::find_user_by_uuid;

pub const SESSION_COOKIE_NAME: &str = "opendesk_session";

pub struct AuthenticatedUser {
    pub user_uuid: Uuid,
    pub username: String,
    pub role: String,
}

pub async fn require_user(
    state: &AppState,
    jar: &CookieJar,
) -> Result<AuthenticatedUser, Response> {
    let Some(cookie) = jar.get(SESSION_COOKIE_NAME) else {
        return Err(Redirect::to("/login").into_response());
    };
    let Ok(session_uuid) = Uuid::parse_str(cookie.value()) else {
        return Err(Redirect::to("/login").into_response());
    };
    let Ok(Some(session)) = find_session(&state.db, session_uuid).await else {
        return Err(Redirect::to("/login").into_response());
    };
    if !session_is_valid(session.expires_at, OffsetDateTime::now_utc()) {
        let _ = delete_session(&state.db, session_uuid).await;
        return Err(Redirect::to("/login").into_response());
    }
    let Ok(Some(user)) = find_user_by_uuid(&state.db, session.user_uuid).await else {
        return Err(Redirect::to("/login").into_response());
    };
    Ok(AuthenticatedUser {
        user_uuid: user.user_uuid,
        username: user.username,
        role: user.role,
    })
}

pub async fn start_session(
    state: &AppState,
    jar: CookieJar,
    user_uuid: Uuid,
) -> Result<(CookieJar, Uuid), sqlx::Error> {
    let session = create_session(&state.db, user_uuid).await?;
    let mut cookie = Cookie::new(SESSION_COOKIE_NAME, session.session_uuid.to_string());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
    if state.cookie_secure {
        cookie.set_secure(true);
    }
    Ok((jar.add(cookie), session.session_uuid))
}

pub async fn end_session(state: &AppState, jar: CookieJar) -> CookieJar {
    if let Some(cookie) = jar.get(SESSION_COOKIE_NAME) {
        if let Ok(session_uuid) = Uuid::parse_str(cookie.value()) {
            let _ = delete_session(&state.db, session_uuid).await;
        }
    }
    let mut removal = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    removal.make_removal();
    jar.remove(removal)
}

pub fn html_error(status: StatusCode, message: &str) -> Response {
    (status, message.to_string()).into_response()
}

pub fn redirect_with_message(path: &str) -> Response {
    Redirect::to(path).into_response()
}

pub fn is_form_post(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/x-www-form-urlencoded"))
        .unwrap_or(false)
}