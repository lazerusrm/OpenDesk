use std::collections::HashSet;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::role::Role;
use time::OffsetDateTime;

use crate::domain::audit_event::AuditEventDraft;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{nav_permissions_for_role, NavPermissions};
use crate::repository::access_groups::list_access_groups_for_user;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::device_visibility::{
    list_user_device_visibility_grants, replace_user_device_visibility_grants,
};
use crate::repository::devices::list_devices;
use crate::repository::users::{disable_user, find_user_by_uuid, DisableUserError};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/{user_uuid}", get(user_detail))
        .route(
            "/users/{user_uuid}/device-visibility-grants",
            axum::routing::post(user_visibility_submit),
        )
        .route(
            "/users/{user_uuid}/disable",
            axum::routing::post(user_disable_submit),
        )
}

#[derive(Template)]
#[template(path = "user_detail.html")]
struct UserDetailView {
    title: String,
    show_nav: bool,
    nav: NavPermissions,
    csrf_token: String,
    user_uuid: String,
    username: String,
    role_display: String,
    activation_state: String,
    can_disable: bool,
    device_options: Vec<UserDeviceOptionView>,
    access_groups: Vec<UserAccessGroupView>,
    error_message: Option<String>,
}

#[derive(Clone)]
struct UserDeviceOptionView {
    device_uuid: String,
    alias: String,
    rustdesk_id_display: String,
    selected: bool,
}

#[derive(Clone)]
struct UserAccessGroupView {
    access_group_uuid: String,
    name: String,
}

#[derive(Deserialize)]
struct DeviceVisibilityForm {
    csrf_token: String,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    device_uuid: Vec<String>,
}

fn parse_uuids(values: &[String]) -> Option<Vec<Uuid>> {
    values
        .iter()
        .map(|value| Uuid::parse_str(value.trim()))
        .collect::<Result<_, _>>()
        .ok()
}

fn deserialize_form_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        One(String),
        Many(Vec<String>),
    }
    Ok(match StringOrVec::deserialize(deserializer)? {
        StringOrVec::One(value) => vec![value],
        StringOrVec::Many(values) => values,
    })
}

async fn user_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_uuid): Path<Uuid>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::UserList).await?;
    render_user_detail(
        &state,
        user_uuid,
        actor.user_uuid,
        &actor.csrf_token,
        nav_permissions_for_role(actor.parsed_role()),
        None,
    )
    .await
}

async fn render_user_detail(
    state: &AppState,
    user_uuid: Uuid,
    actor_user_uuid: Uuid,
    csrf_token: &str,
    nav: NavPermissions,
    error_message: Option<String>,
) -> Result<Response, Response> {
    let user = find_user_by_uuid(&state.db, user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let grants = list_user_device_visibility_grants(&state.db, user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let groups = list_access_groups_for_user(&state.db, user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let grant_ids: HashSet<Uuid> = grants.iter().map(|item| item.device_uuid).collect();
    let role_display = Role::parse(&user.role)
        .map(|role| role.display_label().to_string())
        .unwrap_or_else(|_| user.role.clone());
    let view = UserDetailView {
        title: format!("User · {}", user.username),
        show_nav: true,
        nav,
        csrf_token: csrf_token.to_string(),
        user_uuid: user.user_uuid.to_string(),
        username: user.username,
        role_display,
        can_disable: user.activation_state == "active" && user.user_uuid != actor_user_uuid,
        activation_state: user.activation_state,
        device_options: devices
            .into_iter()
            .map(|device| UserDeviceOptionView {
                device_uuid: device.device_uuid.to_string(),
                alias: device.alias,
                rustdesk_id_display: device.rustdesk_id.unwrap_or_else(|| "-".to_string()),
                selected: grant_ids.contains(&device.device_uuid),
            })
            .collect(),
        access_groups: groups
            .into_iter()
            .map(|group| UserAccessGroupView {
                access_group_uuid: group.access_group_uuid.to_string(),
                name: group.name,
            })
            .collect(),
        error_message,
    };
    Ok(Html(
        view.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?,
    )
    .into_response())
}

#[derive(Deserialize)]
struct UserDisableForm {
    csrf_token: String,
}

async fn user_disable_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_uuid): Path<Uuid>,
    Form(form): Form<UserDisableForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::UserDisable).await?;
    require_csrf(&actor, &form.csrf_token)?;
    match disable_user(
        &state.db,
        actor.user_uuid,
        user_uuid,
        OffsetDateTime::now_utc(),
    )
    .await
    {
        Ok(()) => {
            let audit = AuditEventDraft {
                actor_user_uuid: Some(actor.user_uuid),
                action: "user_disable".to_string(),
                object_type: "user".to_string(),
                object_uuid: Some(user_uuid),
                outcome: "success".to_string(),
                source: "web".to_string(),
                detail: None,
            };
            let _ = insert_audit_event(&state.db, &audit).await;
            Ok(Redirect::to("/users").into_response())
        }
        Err(DisableUserError::CannotDisableSelf | DisableUserError::LastAdmin) => {
            render_user_detail(
                &state,
                user_uuid,
                actor.user_uuid,
                &actor.csrf_token,
                nav_permissions_for_role(actor.parsed_role()),
                Some("this admin account cannot be disabled".to_string()),
            )
            .await
        }
        Err(_) => Err(StatusCode::NOT_FOUND.into_response()),
    }
}

async fn user_visibility_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(user_uuid): Path<Uuid>,
    Form(form): Form<DeviceVisibilityForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::UserUpdate).await?;
    find_user_by_uuid(&state.db, user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    require_csrf(&actor, &form.csrf_token)?;
    let Some(device_uuids) = parse_uuids(&form.device_uuid) else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };
    replace_user_device_visibility_grants(&state.db, user_uuid, &device_uuids)
        .await
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?;
    Ok(Redirect::to(&format!("/users/{user_uuid}")).into_response())
}
