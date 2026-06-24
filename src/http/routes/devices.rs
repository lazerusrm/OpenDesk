use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use askama::Template;
use serde::Deserialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::device::{merge_device_update, validate_device_draft, DeviceDraft};
use crate::domain::device_list::{
    devices_for_default_list, format_notes_display, notes_list_title, rustdesk_id_copy_text,
    DeviceSearchQuery,
};
use crate::domain::tag::format_tag_names_display;
use crate::repository::sites::list_sites;
use crate::repository::tags::{
    list_device_tag_names_map, list_tag_uuids_for_device, set_device_tags,
};
use crate::http::routes::device_export::export_csv_href;
use crate::http::routes::render::render_device_form;
use crate::http::session::{require_user, AuthenticatedUser};
use crate::http::views::{DeviceRowView, DevicesListView};
use crate::time_format::format_last_checkin_display;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::devices::{
    create_device, find_device_by_uuid, list_devices, set_device_archived, update_device,
};

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

#[derive(Deserialize)]
struct SearchQuery {
    term: Option<String>,
}

async fn devices_list(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    let search_term = query.term.unwrap_or_default();
    let search = DeviceSearchQuery {
        term: search_term.clone(),
    };
    let sites = list_sites(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let site_names: std::collections::HashMap<uuid::Uuid, String> = sites
        .iter()
        .map(|site| (site.site_uuid, site.name.clone()))
        .collect();
    let device_tag_names = list_device_tag_names_map(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let listed = devices_for_default_list(&devices, &search, &site_names, &device_tag_names);
    let rows = listed
        .into_iter()
        .map(|device| {
            let tag_names = device_tag_names
                .get(&device.device_uuid)
                .cloned()
                .unwrap_or_default();
            DeviceRowView {
                device_uuid: device.device_uuid.to_string(),
                alias: device.alias.clone(),
                site_display: device
                    .site_uuid
                    .and_then(|uuid| site_names.get(&uuid).cloned())
                    .unwrap_or_else(|| "-".to_string()),
                tags_display: format_tag_names_display(&tag_names),
                notes_display: format_notes_display(device.notes.as_deref()),
                notes_title: notes_list_title(device.notes.as_deref()),
                rustdesk_id_display: device
                    .rustdesk_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                rustdesk_id_copy_text: rustdesk_id_copy_text(device.rustdesk_id.as_deref())
                    .unwrap_or_default(),
                hostname_display: device
                    .hostname
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                last_checkin_display: format_last_checkin_display(device.last_checkin_at.as_deref()),
                archived_display: "no".to_string(),
            }
        })
        .collect();
    let view = DevicesListView {
        title: "Devices".to_string(),
        show_nav: true,
        search_term: search_term.clone(),
        export_csv_href: export_csv_href(&search_term),
        devices: rows,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}

async fn device_new_page(State(state): State<AppState>, jar: CookieJar) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
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
    let _user = require_user(&state, &jar).await?;
    let device = find_device_by_uuid(&state.db, device_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
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
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    .into_response())
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

#[derive(Deserialize)]
pub struct DeviceForm {
    alias: String,
    rustdesk_id: Option<String>,
    hostname: Option<String>,
    owner: Option<String>,
    notes: Option<String>,
    site_uuid: Option<String>,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    tag_uuids: Vec<String>,
}

pub fn parse_tag_uuids_from_form(tag_uuids: &[String]) -> Vec<Uuid> {
    tag_uuids
        .iter()
        .filter_map(|value| Uuid::parse_str(value.trim()).ok())
        .collect()
}

pub fn device_form_to_draft(form: DeviceForm) -> DeviceDraft {
    DeviceDraft {
        rustdesk_id: form.rustdesk_id.filter(|value| !value.trim().is_empty()),
        alias: form.alias,
        hostname: form.hostname.filter(|value| !value.trim().is_empty()),
        owner: form.owner.filter(|value| !value.trim().is_empty()),
        notes: form.notes.filter(|value| !value.trim().is_empty()),
        site_uuid: form
            .site_uuid
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| Uuid::parse_str(value.trim()).ok()),
        ..Default::default()
    }
}

async fn device_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<DeviceForm>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
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
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response());
    }
    let device = create_device(&state.db, &draft)
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
    let user = require_user(&state, &jar).await?;
    let existing = find_device_by_uuid(&state.db, device_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
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

async fn device_archive(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(device_uuid): Path<Uuid>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
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
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    set_device_archived(&state.db, device_uuid, false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    write_device_audit(&state, &user, "device_unarchive", &device_uuid).await;
    Ok(Redirect::to(&format!("/devices/{device_uuid}")).into_response())
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