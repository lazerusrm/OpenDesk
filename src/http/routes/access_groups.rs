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
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{
    nav_permissions_for_role, AccessGroupDetailView, AccessGroupDeviceOptionView,
    AccessGroupDeviceView, AccessGroupMemberView, AccessGroupRowView, AccessGroupUserOptionView,
    AccessGroupsListView, NavPermissions,
};
use crate::repository::access_groups::{
    create_access_group, list_access_group_memberships, list_access_groups,
    list_group_device_visibility_grants, replace_access_group_memberships,
    replace_group_device_visibility_grants,
};
use crate::repository::devices::list_devices;
use crate::repository::users::list_users;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/access-groups",
            get(access_groups_list).post(access_group_create_submit),
        )
        .route(
            "/access-groups/{access_group_uuid}",
            get(access_group_detail),
        )
        .route(
            "/access-groups/{access_group_uuid}/memberships",
            axum::routing::post(access_group_memberships_submit),
        )
        .route(
            "/access-groups/{access_group_uuid}/device-visibility-grants",
            axum::routing::post(access_group_visibility_submit),
        )
}

#[derive(Deserialize)]
struct AccessGroupCreateForm {
    csrf_token: String,
    name: String,
}

#[derive(Deserialize)]
struct UuidReplacementForm {
    csrf_token: String,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    user_uuid: Vec<String>,
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

async fn access_groups_list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupList).await?;
    render_groups_list(
        &state,
        &actor.csrf_token,
        nav_permissions_for_role(actor.parsed_role()),
        None,
    )
    .await
}

async fn render_groups_list(
    state: &AppState,
    csrf_token: &str,
    nav: NavPermissions,
    error_message: Option<String>,
) -> Result<Response, Response> {
    let groups = list_access_groups(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let view = AccessGroupsListView {
        title: "Access groups".to_string(),
        show_nav: true,
        nav,
        csrf_token: csrf_token.to_string(),
        groups: groups
            .into_iter()
            .map(|group| AccessGroupRowView {
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

async fn access_group_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<AccessGroupCreateForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupCreate).await?;
    require_csrf(&actor, &form.csrf_token)?;
    let group = match create_access_group(&state.db, form.name.trim()).await {
        Ok(group) => group,
        Err(error) => {
            return render_groups_list(
                &state,
                &actor.csrf_token,
                nav_permissions_for_role(actor.parsed_role()),
                Some(error.to_string()),
            )
            .await
        }
    };
    Ok(Redirect::to(&format!("/access-groups/{}", group.access_group_uuid)).into_response())
}

async fn access_group_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(access_group_uuid): Path<Uuid>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupView).await?;
    render_group_detail(
        &state,
        access_group_uuid,
        &actor.csrf_token,
        nav_permissions_for_role(actor.parsed_role()),
        None,
    )
    .await
}

async fn render_group_detail(
    state: &AppState,
    access_group_uuid: Uuid,
    csrf_token: &str,
    nav: NavPermissions,
    error_message: Option<String>,
) -> Result<Response, Response> {
    let group =
        crate::repository::access_groups::find_access_group_by_uuid(&state.db, access_group_uuid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let memberships = list_access_group_memberships(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let grants = list_group_device_visibility_grants(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let users = list_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let member_ids: std::collections::HashSet<Uuid> =
        memberships.iter().map(|item| item.user_uuid).collect();
    let grant_ids: std::collections::HashSet<Uuid> =
        grants.iter().map(|item| item.device_uuid).collect();
    let view = AccessGroupDetailView {
        title: format!("Access group · {}", group.name),
        show_nav: true,
        nav,
        csrf_token: csrf_token.to_string(),
        access_group_uuid: group.access_group_uuid.to_string(),
        name: group.name,
        members: users
            .iter()
            .filter(|user| member_ids.contains(&user.user_uuid))
            .map(|user| AccessGroupMemberView {
                username: user.username.clone(),
                user_uuid: user.user_uuid.to_string(),
            })
            .collect(),
        devices: devices
            .iter()
            .filter(|device| grant_ids.contains(&device.device_uuid))
            .map(|device| AccessGroupDeviceView {
                alias: device.alias.clone(),
                device_uuid: device.device_uuid.to_string(),
            })
            .collect(),
        user_options: users
            .into_iter()
            .map(|user| AccessGroupUserOptionView {
                username: user.username,
                user_uuid: user.user_uuid.to_string(),
                selected: member_ids.contains(&user.user_uuid),
            })
            .collect(),
        device_options: devices
            .into_iter()
            .map(|device| AccessGroupDeviceOptionView {
                alias: device.alias,
                device_uuid: device.device_uuid.to_string(),
                selected: grant_ids.contains(&device.device_uuid),
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

async fn access_group_memberships_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(access_group_uuid): Path<Uuid>,
    Form(form): Form<UuidReplacementForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupUpdate).await?;
    crate::repository::access_groups::find_access_group_by_uuid(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    require_csrf(&actor, &form.csrf_token)?;
    let Some(user_uuids) = parse_uuids(&form.user_uuid) else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };
    replace_access_group_memberships(&state.db, access_group_uuid, &user_uuids)
        .await
        .map_err(|error| {
            if matches!(
                error,
                crate::repository::access_groups::AccessGroupRepositoryError::Database(
                    sqlx::Error::RowNotFound
                )
            ) {
                StatusCode::NOT_FOUND.into_response()
            } else {
                StatusCode::NOT_FOUND.into_response()
            }
        })?;
    Ok(Redirect::to(&format!("/access-groups/{access_group_uuid}")).into_response())
}

async fn access_group_visibility_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(access_group_uuid): Path<Uuid>,
    Form(form): Form<UuidReplacementForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupUpdate).await?;
    crate::repository::access_groups::find_access_group_by_uuid(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    require_csrf(&actor, &form.csrf_token)?;
    let Some(device_uuids) = parse_uuids(&form.device_uuid) else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };
    replace_group_device_visibility_grants(&state.db, access_group_uuid, &device_uuids)
        .await
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?;
    Ok(Redirect::to(&format!("/access-groups/{access_group_uuid}")).into_response())
}
