use askama::Template;
use axum::{
    extract::State,
    http::{header, HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::auth::password_meets_policy;
use crate::config::{persist_public_base_url, public_base_url_is_valid};
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::server_config::{validate_server_config, ServerConfig};
use crate::http::session::{constant_time_equal, require_same_origin};
use crate::http::views::NavPermissions;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::server_config::save_server_config;
use crate::repository::users::{count_users, create_user};

const SETUP_CSRF_COOKIE: &str = "opendesk_setup_csrf";
const ADMIN_USERNAME: &str = "admin";

pub fn routes() -> Router<AppState> {
    Router::new().route("/setup", get(setup_page).post(setup_submit))
}

#[derive(Template)]
#[template(path = "setup.html")]
struct SetupView {
    title: String,
    show_nav: bool,
    nav: NavPermissions,
    csrf_token: String,
    username: String,
    error_message: Option<String>,
    public_base_url: String,
    id_server: String,
    relay_server: String,
    api_server: String,
    public_key: String,
}

#[derive(Deserialize)]
struct SetupForm {
    password: String,
    confirm_password: String,
    public_base_url: String,
    id_server: String,
    relay_server: String,
    api_server: String,
    public_key: String,
    #[serde(default)]
    csrf_token: String,
}

fn setup_view(
    csrf_token: String,
    error_message: Option<String>,
    public_base_url: String,
    id_server: String,
    relay_server: String,
    api_server: String,
    public_key: String,
) -> SetupView {
    SetupView {
        title: "Set up OpenDesk".to_string(),
        show_nav: false,
        nav: NavPermissions::NONE,
        csrf_token,
        username: ADMIN_USERNAME.to_string(),
        error_message,
        public_base_url,
        id_server,
        relay_server,
        api_server,
        public_key,
    }
}

fn html_page(view: SetupView) -> Result<Response, Response> {
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let mut response = Html(html).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex"),
    );
    Ok(response)
}

fn ensure_csrf(jar: CookieJar, secure: bool) -> (CookieJar, String) {
    if let Some(value) = jar
        .get(SETUP_CSRF_COOKIE)
        .map(|cookie| cookie.value().to_string())
        .filter(|value| !value.is_empty())
    {
        return (jar, value);
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let value = hex::encode(bytes);
    let mut cookie = Cookie::new(SETUP_CSRF_COOKIE, value.clone());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/setup");
    cookie.set_secure(secure);
    (jar.add(cookie), value)
}

fn csrf_ok(jar: &CookieJar, provided: &str) -> bool {
    jar.get(SETUP_CSRF_COOKIE)
        .is_some_and(|cookie| constant_time_equal(cookie.value().as_bytes(), provided.as_bytes()))
}

async fn setup_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let users = count_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if users > 0 {
        return Ok(Redirect::to("/login").into_response());
    }
    let (jar, csrf) = ensure_csrf(jar, state.cookie_secure);
    Ok((
        jar,
        html_page(setup_view(
            csrf,
            None,
            state.public_base_url(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ))?,
    )
        .into_response())
}

async fn setup_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Form(form): Form<SetupForm>,
) -> Result<Response, Response> {
    let users = count_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if users > 0 {
        return Ok(Redirect::to("/login").into_response());
    }
    let (jar, csrf) = ensure_csrf(jar, state.cookie_secure);
    let rediscover = |error: String| {
        html_page(setup_view(
            csrf.clone(),
            Some(error),
            form.public_base_url.clone(),
            form.id_server.clone(),
            form.relay_server.clone(),
            form.api_server.clone(),
            form.public_key.clone(),
        ))
    };
    if !require_same_origin(&headers, &state.public_base_url()) || !csrf_ok(&jar, &form.csrf_token)
    {
        return Ok((jar, rediscover("Try again from this page.".to_string())?).into_response());
    }
    if form.password != form.confirm_password {
        return Ok((
            jar,
            rediscover("new password and confirmation do not match".to_string())?,
        )
            .into_response());
    }
    if !password_meets_policy(&form.password) {
        return Ok((
            jar,
            rediscover("password must be at least 8 characters".to_string())?,
        )
            .into_response());
    }
    if !public_base_url_is_valid(&form.public_base_url) {
        return Ok((
            jar,
            rediscover("public URL must be an http or https origin".to_string())?,
        )
            .into_response());
    }
    let config = ServerConfig {
        id_server: form.id_server.trim().to_string(),
        relay_server: form.relay_server.trim().to_string(),
        api_server: form.api_server.trim().to_string(),
        public_key: form.public_key.trim().to_string(),
    };
    if let Err(error) = validate_server_config(&config) {
        return Ok((jar, rediscover(error.to_string())?).into_response());
    }
    let user = create_user(&state.db, ADMIN_USERNAME, form.password.trim(), "admin")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    save_server_config(&state.db, &config, Some(user.user_uuid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    persist_public_base_url(&state.data_dir, form.public_base_url.trim())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    state.set_public_base_url(form.public_base_url.trim().to_string());
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "first_run_setup".to_string(),
        object_type: "user".to_string(),
        object_uuid: Some(user.user_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok((jar, Redirect::to("/login")).into_response())
}
