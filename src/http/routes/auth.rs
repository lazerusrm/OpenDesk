use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::auth;
use crate::domain::audit_event::AuditEventDraft;
use crate::http::routes::render::render_login;
use crate::http::session::{end_session, start_session};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::users::find_user_by_username;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async { Redirect::to("/devices") }))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout))
}

async fn login_page() -> impl IntoResponse {
    render_login(None)
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<Response, StatusCode> {
    let user = find_user_by_username(&state.db, form.username.trim())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(user) = user else {
        return Ok(render_login(Some("Invalid username or password".to_string())).into_response());
    };
    if auth::verify_password(&form.password, &user.password_hash).is_err() {
        return Ok(render_login(Some("Invalid username or password".to_string())).into_response());
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
    Ok((jar, Redirect::to("/devices")).into_response())
}

async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let jar = end_session(&state, jar).await;
    (jar, Redirect::to("/login"))
}