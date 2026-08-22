use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::access_policy::{is_action_allowed_for_values, Action};
use crate::domain::role::Role;
use crate::domain::session::session_is_valid;
use crate::repository::sessions::{create_session, delete_session, find_session};
use crate::repository::users::find_user_by_uuid;

pub const SESSION_COOKIE_NAME: &str = "opendesk_session";
pub const CSRF_COOKIE_NAME: &str = "opendesk_csrf";
pub const CSRF_FIELD_NAME: &str = "csrf_token";

pub struct AuthenticatedUser {
    pub user_uuid: Uuid,
    pub username: String,
    pub role: String,
    pub csrf_token: String,
}

impl AuthenticatedUser {
    pub fn parsed_role(&self) -> Role {
        Role::parse(&self.role).unwrap_or(Role::ReadOnly)
    }
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
    if session.csrf_token.is_empty()
        || !session_is_valid(session.expires_at, OffsetDateTime::now_utc())
    {
        let _ = delete_session(&state.db, session_uuid).await;
        return Err(Redirect::to("/login").into_response());
    }
    let Ok(Some(user)) = find_user_by_uuid(&state.db, session.user_uuid).await else {
        return Err(Redirect::to("/login").into_response());
    };
    if user.activation_state != "active" {
        let _ = delete_session(&state.db, session_uuid).await;
        return Err(Redirect::to("/login").into_response());
    }
    Ok(AuthenticatedUser {
        user_uuid: user.user_uuid,
        username: user.username,
        role: user.role,
        csrf_token: session.csrf_token,
    })
}

pub async fn require_action(
    state: &AppState,
    jar: &CookieJar,
    action: Action,
) -> Result<AuthenticatedUser, Response> {
    let user = require_user(state, jar).await?;
    if !is_action_allowed_for_values(&user.role, action.as_str()) {
        return Err(forbidden_response());
    }
    Ok(user)
}

/// Authenticated users with admin or operator role (mutations and exports).
pub async fn require_mutator(
    state: &AppState,
    jar: &CookieJar,
) -> Result<AuthenticatedUser, Response> {
    let user = require_user(state, jar).await?;
    if !user.parsed_role().can_mutate() {
        return Err(forbidden_response());
    }
    Ok(user)
}

/// Authenticated users with admin role only.
pub async fn require_admin(
    state: &AppState,
    jar: &CookieJar,
) -> Result<AuthenticatedUser, Response> {
    let user = require_user(state, jar).await?;
    if !user.parsed_role().can_admin() {
        return Err(forbidden_response());
    }
    Ok(user)
}

pub fn require_csrf(user: &AuthenticatedUser, provided: &str) -> Result<(), Response> {
    if constant_time_equal(user.csrf_token.as_bytes(), provided.as_bytes()) {
        Ok(())
    } else {
        Err(forbidden_response())
    }
}

pub fn require_present_same_origin(headers: &HeaderMap, public_base_url: &str) -> bool {
    let expected = origin_from_url(public_base_url);
    let candidate = headers
        .get("origin")
        .or_else(|| headers.get("referer"))
        .and_then(|value| value.to_str().ok())
        .and_then(origin_from_url);
    candidate.is_some() && candidate == expected
}

pub fn require_same_origin(headers: &HeaderMap, public_base_url: &str) -> bool {
    let expected = origin_from_url(public_base_url);
    let candidate = headers
        .get("origin")
        .or_else(|| headers.get("referer"))
        .and_then(|value| value.to_str().ok())
        .and_then(origin_from_url);
    candidate.is_none() || candidate == expected
}

fn origin_from_url(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('/');
    let scheme_end = value.find("://")?;
    let authority = value[scheme_end + 3..].split('/').next()?;
    if authority.is_empty() {
        return None;
    }
    Some(format!("{}://{}", &value[..scheme_end], authority).to_ascii_lowercase())
}

pub(crate) fn constant_time_equal(expected: &[u8], provided: &[u8]) -> bool {
    let mut difference = (expected.len() ^ provided.len()) as u8;
    for index in 0..expected.len().max(provided.len()) {
        difference |=
            expected.get(index).copied().unwrap_or(0) ^ provided.get(index).copied().unwrap_or(0);
    }
    difference == 0
}

pub fn forbidden_response() -> Response {
    html_error(StatusCode::FORBIDDEN, "Forbidden")
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
    cookie.set_secure(state.cookie_secure);
    let csrf_cookie = Cookie::build((CSRF_COOKIE_NAME, session.csrf_token.clone()))
        .http_only(false)
        .same_site(SameSite::Lax)
        .path("/")
        .secure(state.cookie_secure)
        .build();
    Ok((jar.add(cookie).add(csrf_cookie), session.session_uuid))
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
        .secure(state.cookie_secure)
        .build();
    removal.make_removal();
    let mut csrf_removal = Cookie::build((CSRF_COOKIE_NAME, ""))
        .path("/")
        .same_site(SameSite::Lax)
        .secure(state.cookie_secure)
        .build();
    csrf_removal.make_removal();
    jar.remove(removal).remove(csrf_removal)
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

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::{constant_time_equal, require_present_same_origin};

    #[test]
    fn constant_time_equal_requires_exact_token() {
        assert!(constant_time_equal(b"token", b"token"));
        assert!(!constant_time_equal(b"token", b"Token"));
        assert!(!constant_time_equal(b"token", b"token-extra"));
    }

    #[test]
    fn same_origin_requires_matching_present_header() {
        let mut headers = HeaderMap::new();
        assert!(!require_present_same_origin(
            &headers,
            "https://admin.example"
        ));
        headers.insert("origin", "https://evil.example".parse().unwrap());
        assert!(!require_present_same_origin(
            &headers,
            "https://admin.example"
        ));
        headers.insert("origin", "https://admin.example".parse().unwrap());
        assert!(require_present_same_origin(
            &headers,
            "https://admin.example"
        ));
        headers.remove("origin");
        headers.insert("referer", "https://admin.example/login".parse().unwrap());
        assert!(require_present_same_origin(
            &headers,
            "https://admin.example"
        ));
    }
}
