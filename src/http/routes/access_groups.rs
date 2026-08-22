use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Query, State},
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
    nav_permissions_for_role, AccessGroupRowView, AccessGroupsListView, NavPermissions,
};
use crate::repository::access_groups::{
    create_access_group, list_access_group_memberships, list_access_groups,
    list_group_device_visibility_grants,
};
use crate::repository::devices::list_devices;
use crate::repository::users::list_users;

#[path = "access_group_detail.rs"]
mod access_group_detail;
#[path = "access_group_display.rs"]
mod access_group_display;
use access_group_display::{
    count_label, device_label, join_or_empty, rustdesk_id_text, AccessGroupCreateForm,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/access-groups",
            get(access_groups_list).post(access_group_create_submit),
        )
        .route(
            "/access-groups/{access_group_uuid}",
            get(access_group_detail::access_group_detail),
        )
        .route(
            "/access-groups/{access_group_uuid}/memberships",
            axum::routing::post(access_group_detail::access_group_memberships_submit),
        )
        .route(
            "/access-groups/{access_group_uuid}/device-visibility-grants",
            axum::routing::post(access_group_detail::access_group_visibility_submit),
        )
        .route(
            "/access-groups/{access_group_uuid}/access-grants",
            axum::routing::post(access_group_detail::access_group_access_submit),
        )
}

#[derive(Deserialize)]
struct AccessGroupsListQuery {
    view: Option<String>,
}

#[derive(Clone, Copy)]
enum GroupsListTab {
    All,
    Users,
    Devices,
}

impl GroupsListTab {
    fn parse(value: &str) -> Self {
        match value.trim() {
            "users" => Self::Users,
            "devices" => Self::Devices,
            _ => Self::All,
        }
    }
}

async fn load_group_rows(state: &AppState) -> Result<Vec<AccessGroupRowView>, Response> {
    let groups = list_access_groups(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let users = list_users(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let user_names: HashMap<Uuid, String> = users
        .into_iter()
        .map(|user| (user.user_uuid, user.username))
        .collect();
    let device_labels: HashMap<Uuid, String> = devices
        .into_iter()
        .map(|device| {
            (
                device.device_uuid,
                device_label(&device.alias, &rustdesk_id_text(&device.rustdesk_id)),
            )
        })
        .collect();
    let mut rows = Vec::with_capacity(groups.len());
    for group in groups {
        let memberships = list_access_group_memberships(&state.db, group.access_group_uuid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
        let grants = list_group_device_visibility_grants(&state.db, group.access_group_uuid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
        let mut member_names: Vec<String> = memberships
            .iter()
            .filter_map(|item| user_names.get(&item.user_uuid).cloned())
            .collect();
        member_names.sort();
        let mut device_names: Vec<String> = grants
            .iter()
            .filter_map(|item| device_labels.get(&item.device_uuid).cloned())
            .collect();
        device_names.sort();
        let member_count = member_names.len();
        let device_count = device_names.len();
        rows.push(AccessGroupRowView {
            access_group_uuid: group.access_group_uuid.to_string(),
            name: group.name,
            member_count,
            device_count,
            member_count_label: count_label(member_count, "member", "members"),
            device_count_label: count_label(device_count, "device", "devices"),
            member_names_display: join_or_empty(&member_names, "No members"),
            device_names_display: join_or_empty(&device_names, "No devices"),
        });
    }
    Ok(rows)
}

async fn access_groups_list(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<AccessGroupsListQuery>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AccessGroupList).await?;
    render_groups_list(
        &state,
        &actor.csrf_token,
        nav_permissions_for_role(actor.parsed_role()),
        GroupsListTab::parse(query.view.as_deref().unwrap_or("")),
        None,
    )
    .await
}

async fn render_groups_list(
    state: &AppState,
    csrf_token: &str,
    nav: NavPermissions,
    tab: GroupsListTab,
    error_message: Option<String>,
) -> Result<Response, Response> {
    let groups = load_group_rows(state).await?;
    let view = AccessGroupsListView {
        title: "Access groups".to_string(),
        show_nav: true,
        nav,
        csrf_token: csrf_token.to_string(),
        groups,
        show_all_groups: matches!(tab, GroupsListTab::All),
        show_user_memberships: matches!(tab, GroupsListTab::Users),
        show_device_visibility: matches!(tab, GroupsListTab::Devices),
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
                GroupsListTab::All,
                Some(error.to_string()),
            )
            .await
        }
    };
    Ok(Redirect::to(&format!("/access-groups/{}", group.access_group_uuid)).into_response())
}
