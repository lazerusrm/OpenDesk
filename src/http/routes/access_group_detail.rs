use std::collections::HashSet;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use super::access_group_display::{
    count_label, parse_uuids, rustdesk_id_text, AccessGroupAccessForm, UuidReplacementForm,
};
use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{
    nav_permissions_for_role, AccessGroupDetailView, AccessGroupDeviceOptionView,
    AccessGroupDeviceView, AccessGroupGroupOptionView, AccessGroupMemberView,
    AccessGroupUserOptionView, NavPermissions,
};
use crate::repository::access_groups::{
    list_access_group_memberships, list_access_groups, list_group_device_visibility_grants,
    replace_access_group_memberships, replace_group_device_visibility_grants,
};
use crate::repository::device_visibility::{
    list_incoming_access_group_access_grants, list_outgoing_access_group_access_grants,
    replace_outgoing_access_group_access_grants, DeviceVisibilityRepositoryError,
};
use crate::repository::devices::list_devices;
use crate::repository::users::list_users;

pub(super) async fn access_group_detail(
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

pub(super) async fn render_group_detail(
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
    let outgoing = list_outgoing_access_group_access_grants(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let incoming = list_incoming_access_group_access_grants(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let users = list_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let groups = list_access_groups(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let member_ids: HashSet<Uuid> = memberships.iter().map(|item| item.user_uuid).collect();
    let grant_ids: HashSet<Uuid> = grants.iter().map(|item| item.device_uuid).collect();
    let outgoing_ids: HashSet<Uuid> = outgoing
        .iter()
        .map(|item| item.outgoing_access_group_uuid)
        .collect();
    let incoming_ids: HashSet<Uuid> = incoming
        .iter()
        .map(|item| item.incoming_access_group_uuid)
        .collect();
    let mut incoming_names: Vec<String> = groups
        .iter()
        .filter(|item| incoming_ids.contains(&item.access_group_uuid))
        .map(|item| item.name.clone())
        .collect();
    incoming_names.sort();
    let view = AccessGroupDetailView {
        title: format!("Access group · {}", group.name),
        show_nav: true,
        nav,
        csrf_token: csrf_token.to_string(),
        access_group_uuid: group.access_group_uuid.to_string(),
        name: group.name,
        member_count_label: count_label(member_ids.len(), "member", "members"),
        device_count_label: count_label(grant_ids.len(), "visible device", "visible devices"),
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
                rustdesk_id: rustdesk_id_text(&device.rustdesk_id),
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
                rustdesk_id: rustdesk_id_text(&device.rustdesk_id),
                alias: device.alias,
                device_uuid: device.device_uuid.to_string(),
                selected: grant_ids.contains(&device.device_uuid),
            })
            .collect(),
        group_options: groups
            .into_iter()
            .filter(|item| item.access_group_uuid != access_group_uuid)
            .map(|item| AccessGroupGroupOptionView {
                selected: outgoing_ids.contains(&item.access_group_uuid),
                access_group_uuid: item.access_group_uuid.to_string(),
                name: item.name,
            })
            .collect(),
        incoming_group_names_display: if incoming_names.is_empty() {
            "No other groups can see devices granted to this group.".to_string()
        } else {
            format!(
                "Groups that can see devices granted to this group: {}",
                incoming_names.join(", ")
            )
        },
        error_message,
    };
    Ok(Html(
        view.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?,
    )
    .into_response())
}

pub(super) async fn access_group_memberships_submit(
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
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?;
    Ok(Redirect::to(&format!("/access-groups/{access_group_uuid}")).into_response())
}

pub(super) async fn access_group_visibility_submit(
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

pub(super) async fn access_group_access_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(access_group_uuid): Path<Uuid>,
    Form(form): Form<AccessGroupAccessForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupUpdate).await?;
    crate::repository::access_groups::find_access_group_by_uuid(&state.db, access_group_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    require_csrf(&actor, &form.csrf_token)?;
    let Some(outgoing) = parse_uuids(&form.outgoing_access_group_uuid) else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };
    match replace_outgoing_access_group_access_grants(&state.db, access_group_uuid, &outgoing).await
    {
        Ok(()) => Ok(Redirect::to(&format!("/access-groups/{access_group_uuid}")).into_response()),
        Err(DeviceVisibilityRepositoryError::SelfAccessGrant) => {
            render_group_detail(
                &state,
                access_group_uuid,
                &actor.csrf_token,
                nav_permissions_for_role(actor.parsed_role()),
                Some("An access group cannot grant access to itself.".to_string()),
            )
            .await
        }
        Err(DeviceVisibilityRepositoryError::DuplicateUuid(_))
        | Err(DeviceVisibilityRepositoryError::Database(sqlx::Error::RowNotFound)) => {
            Err(StatusCode::NOT_FOUND.into_response())
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    }
}
