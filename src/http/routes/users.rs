use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::role::{validate_role_value, Role};
use crate::domain::user::validate_username;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{nav_permissions_for_role, UserRowView, UsersListView};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::users::{create_user, list_users};

pub fn routes() -> Router<AppState> {
    Router::new().route("/users", get(users_list).post(user_create_submit))
}

async fn users_list(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::UserList).await?;
    Ok(
        render_users_page(&state, None, &actor.csrf_token, actor.parsed_role())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response(),
    )
}

#[derive(Deserialize)]
struct UserCreateForm {
    csrf_token: String,
    username: String,
    password: String,
    role: String,
}

async fn user_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<UserCreateForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::UserCreate).await?;
    require_csrf(&actor, &form.csrf_token)?;
    if let Err(error) = validate_username(&form.username) {
        return Ok(render_users_page(
            &state,
            Some(error.to_string()),
            &actor.csrf_token,
            actor.parsed_role(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    if form.password.trim().len() < 8 {
        return Ok(render_users_page(
            &state,
            Some("password must be at least 8 characters".to_string()),
            &actor.csrf_token,
            actor.parsed_role(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    let role = match validate_role_value(&form.role) {
        Ok(role) => role,
        Err(_) => {
            return Ok(render_users_page(
                &state,
                Some("role must be admin, operator, or read_only".to_string()),
                &actor.csrf_token,
                actor.parsed_role(),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response());
        }
    };
    let created = match create_user(
        &state.db,
        form.username.trim(),
        form.password.trim(),
        role.as_str(),
    )
    .await
    {
        Ok(user) => user,
        Err(error) => {
            let message = if error.to_string().contains("UNIQUE") {
                "username already exists".to_string()
            } else {
                "failed to create user".to_string()
            };
            return Ok(render_users_page(
                &state,
                Some(message),
                &actor.csrf_token,
                actor.parsed_role(),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response());
        }
    };
    let audit = AuditEventDraft {
        actor_user_uuid: Some(actor.user_uuid),
        action: "user_create".to_string(),
        object_type: "user".to_string(),
        object_uuid: Some(created.user_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: Some(serde_json::json!({
            "username": created.username,
            "role": created.role,
        })),
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(Redirect::to("/users").into_response())
}

async fn render_users_page(
    state: &AppState,
    error_message: Option<String>,
    csrf_token: &str,
    role: crate::domain::role::Role,
) -> Result<Html<String>, sqlx::Error> {
    let users = list_users(&state.db).await?;
    let rows = users
        .into_iter()
        .map(|user| {
            let role_label = Role::parse(&user.role)
                .map(|role| role.display_label().to_string())
                .unwrap_or_else(|_| user.role.clone());
            UserRowView {
                user_uuid: user.user_uuid.to_string(),
                username: user.username,
                role_display: role_label,
            }
        })
        .collect();
    let view = UsersListView {
        title: "Users".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(role),
        csrf_token: csrf_token.to_string(),
        users: rows,
        error_message,
    };
    Ok(Html(view.render().expect("render users list")))
}
