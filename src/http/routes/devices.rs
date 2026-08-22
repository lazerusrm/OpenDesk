#[path = "devices_form.rs"]
mod devices_form;

use devices_form::{device_form_to_draft, parse_tag_uuids_from_form, DeviceForm};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::device::{merge_device_update, validate_device_draft, Device, DeviceDraft};
use crate::http::routes::render::render_device_form;
use crate::http::session::{require_action, require_csrf, AuthenticatedUser};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::device_visibility::is_device_visible_to_user;
use crate::repository::devices::{
    create_device_with_visibility, find_device_by_uuid, set_device_archived, update_device,
};
use crate::repository::tags::{list_tag_uuids_for_device, set_device_tags};

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(super::device_export::routes())
        .route("/devices", get(devices_list).post(device_create_submit))
        .route("/devices/new", get(device_new_page))
        .route(
            "/devices/{device_uuid}",
            get(device_edit_page).post(device_update_submit),
        )
        .route("/devices/{device_uuid}/archive", post(device_archive))
        .route("/devices/{device_uuid}/unarchive", post(device_unarchive))
}

#[path = "devices_list_page.rs"]
mod devices_list_page;
use devices_list_page::devices_list;

async fn find_visible_device(
    state: &AppState,
    user_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<Device, Response> {
    let device = find_device_by_uuid(&state.db, device_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    if !is_device_visible_to_user(&state.db, user_uuid, device_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    Ok(device)
}

async fn device_new_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceCreate).await?;
    Ok(render_device_form(
        &state,
        "New device",
        "/devices",
        Uuid::nil(),
        DeviceDraft::default(),
        &[],
        None,
        false,
        false,
        &user.csrf_token,
        user.parsed_role(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    .into_response())
}

async fn device_edit_page(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(device_uuid): Path<Uuid>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceView).await?;
    let device = find_visible_device(&state, user.user_uuid, device_uuid).await?;
    let draft = DeviceDraft {
        rustdesk_id: device.rustdesk_id,
        alias: device.alias,
        hostname: device.hostname,
        os_family: device.os_family,
        os_version: device.os_version,
        architecture: device.architecture,
        rustdesk_version: device.rustdesk_version,
        site_uuid: device.site_uuid,
        owner: device.owner,
        notes: device.notes,
    };
    let selected_tag_uuids = list_tag_uuids_for_device(&state.db, device_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(render_device_form(
        &state,
        "Edit device",
        &format!("/devices/{device_uuid}"),
        device_uuid,
        draft,
        &selected_tag_uuids,
        None,
        !device.archived,
        device.archived,
        &user.csrf_token,
        user.parsed_role(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    .into_response())
}

async fn device_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<DeviceForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceCreate).await?;
    require_csrf(&user, &form.csrf_token)?;
    let tag_uuids = parse_tag_uuids_from_form(&form.tag_uuids);
    let draft = device_form_to_draft(form);
    if let Err(error) = validate_device_draft(&draft) {
        return Ok(render_device_form(
            &state,
            "New device",
            "/devices",
            Uuid::nil(),
            draft,
            &tag_uuids,
            Some(error.to_string()),
            false,
            false,
            &user.csrf_token,
            user.parsed_role(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    let device = create_device_with_visibility(&state.db, &draft, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    set_device_tags(&state.db, device.device_uuid, &tag_uuids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    write_device_audit(&state, &user, "device_create", &device.device_uuid).await;
    Ok(Redirect::to(&format!("/devices/{}", device.device_uuid)).into_response())
}

async fn device_update_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(device_uuid): Path<Uuid>,
    Form(form): Form<DeviceForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceUpdate).await?;
    let existing = find_visible_device(&state, user.user_uuid, device_uuid).await?;
    require_csrf(&user, &form.csrf_token)?;
    let tag_uuids = parse_tag_uuids_from_form(&form.tag_uuids);
    let draft = merge_device_update(device_form_to_draft(form), &existing);
    if let Err(error) = validate_device_draft(&draft) {
        return Ok(render_device_form(
            &state,
            "Edit device",
            &format!("/devices/{device_uuid}"),
            device_uuid,
            draft,
            &tag_uuids,
            Some(error.to_string()),
            !existing.archived,
            existing.archived,
            &user.csrf_token,
            user.parsed_role(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    let device = update_device(&state.db, device_uuid, &draft)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    set_device_tags(&state.db, device_uuid, &tag_uuids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    write_device_audit(&state, &user, "device_update", &device.device_uuid).await;
    Ok(Redirect::to(&format!("/devices/{}", device.device_uuid)).into_response())
}

#[derive(Deserialize)]
struct ArchiveForm {
    csrf_token: String,
}

async fn device_archive(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(device_uuid): Path<Uuid>,
    Form(form): Form<ArchiveForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceArchive).await?;
    let _existing = find_visible_device(&state, user.user_uuid, device_uuid).await?;
    require_csrf(&user, &form.csrf_token)?;
    set_device_archived(&state.db, device_uuid, true)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    write_device_audit(&state, &user, "device_archive", &device_uuid).await;
    Ok(Redirect::to(&format!("/devices/{device_uuid}")).into_response())
}

async fn device_unarchive(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(device_uuid): Path<Uuid>,
    Form(form): Form<ArchiveForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceUnarchive).await?;
    let _existing = find_visible_device(&state, user.user_uuid, device_uuid).await?;
    require_csrf(&user, &form.csrf_token)?;
    set_device_archived(&state.db, device_uuid, false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    write_device_audit(&state, &user, "device_unarchive", &device_uuid).await;
    Ok(Redirect::to(&format!("/devices/{device_uuid}")).into_response())
}

async fn write_device_audit(
    state: &AppState,
    user: &AuthenticatedUser,
    action: &str,
    device_uuid: &Uuid,
) {
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: action.to_string(),
        object_type: "device".to_string(),
        object_uuid: Some(*device_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}

#[cfg(test)]
mod form_tests {
    use super::DeviceForm;

    #[test]
    fn deserializes_single_tag_uuid_field() {
        let body = format!(
            "alias=Tagged+Workstation&tag_uuids={}",
            uuid::Uuid::new_v4()
        );
        let form: DeviceForm = serde_urlencoded::from_str(&body).expect("deserialize form");
        assert_eq!(form.tag_uuids.len(), 1);
    }

    #[test]
    fn list_query_defaults_hide_archived_and_any_seen() {
        assert_eq!(super::devices_list_page::parse_archived_filter(None), "0");
        assert_eq!(
            super::devices_list_page::parse_archived_filter(Some("")),
            "0"
        );
        assert_eq!(
            super::devices_list_page::parse_archived_filter(Some("nope")),
            "0"
        );
        assert_eq!(
            super::devices_list_page::parse_archived_filter(Some("1")),
            "1"
        );
        assert_eq!(
            super::devices_list_page::parse_archived_filter(Some("all")),
            "all"
        );
        assert_eq!(super::devices_list_page::parse_seen_filter(None), "any");
        assert_eq!(super::devices_list_page::parse_seen_filter(Some("")), "any");
        assert_eq!(
            super::devices_list_page::parse_seen_filter(Some("recent")),
            "recent"
        );
        assert_eq!(
            super::devices_list_page::parse_seen_filter(Some("online")),
            "any"
        );
    }
}
