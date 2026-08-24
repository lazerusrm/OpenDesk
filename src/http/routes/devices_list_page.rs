use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::connection_helper::{
    explicit_server_helper_for_device, generate_default_server_helper,
};
use crate::domain::device_list::{
    device_matches_search_with_metadata, format_notes_display, notes_list_title, recently_seen,
    rustdesk_id_copy_text, DeviceSearchQuery, RECENTLY_SEEN_WINDOW,
};
use crate::domain::server_config::default_server_config;
use crate::domain::tag::format_tag_names_display;
use crate::http::routes::device_export::export_csv_href;
use crate::http::session::require_action;
use crate::http::views::{nav_permissions_for_role, DeviceRowView, DevicesListView};
use crate::repository::device_visibility::list_visible_device_uuids_for_user;
use crate::repository::devices::list_devices;
use crate::repository::server_config::load_server_config;
use crate::repository::sites::list_sites;
use crate::repository::tags::list_device_tag_names_map;
use crate::time_format::format_last_checkin_display;

#[derive(Deserialize)]
pub(super) struct SearchQuery {
    term: Option<String>,
    archived: Option<String>,
    seen: Option<String>,
}

pub(super) fn parse_archived_filter(value: Option<&str>) -> &'static str {
    match value.map(str::trim) {
        Some("1") => "1",
        Some("all") => "all",
        _ => "0",
    }
}

pub(super) fn parse_seen_filter(value: Option<&str>) -> &'static str {
    match value.map(str::trim) {
        Some("recent") => "recent",
        _ => "any",
    }
}

pub(super) async fn devices_list(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeviceList).await?;
    let visible_device_uuids = list_visible_device_uuids_for_user(&state.db, user.user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let search_term = query.term.unwrap_or_default();
    let archived_filter = parse_archived_filter(query.archived.as_deref());
    let seen_filter = parse_seen_filter(query.seen.as_deref());
    let search = DeviceSearchQuery {
        term: search_term.clone(),
    };
    let now = OffsetDateTime::now_utc();
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
    let server_config = load_server_config(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .unwrap_or_else(default_server_config);
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let visible_devices: Vec<_> = devices
        .into_iter()
        .filter(|device| visible_device_uuids.contains(&device.device_uuid))
        .collect();
    let empty_inventory = visible_devices.is_empty();
    let listed: Vec<_> = visible_devices
        .iter()
        .filter(|device| match archived_filter {
            "1" => device.archived,
            "all" => true,
            _ => !device.archived,
        })
        .filter(|device| {
            seen_filter != "recent"
                || recently_seen(device.last_checkin_at.as_deref(), now, RECENTLY_SEEN_WINDOW)
        })
        .filter(|device| {
            let site_name = device
                .site_uuid
                .and_then(|uuid| site_names.get(&uuid).map(String::as_str));
            let tag_names: Vec<&str> = device_tag_names
                .get(&device.device_uuid)
                .map(|names| names.iter().map(String::as_str).collect())
                .unwrap_or_default();
            device_matches_search_with_metadata(device, &search, site_name, &tag_names)
        })
        .collect();
    let filter_empty = !empty_inventory && listed.is_empty();
    let rows = listed
        .into_iter()
        .map(|device| {
            let tag_names = device_tag_names
                .get(&device.device_uuid)
                .cloned()
                .unwrap_or_default();
            let device_recently_seen =
                recently_seen(device.last_checkin_at.as_deref(), now, RECENTLY_SEEN_WINDOW);
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
                default_helper_copy_text: generate_default_server_helper(
                    device.rustdesk_id.as_deref(),
                )
                .unwrap_or_default(),
                explicit_helper_copy_text: explicit_server_helper_for_device(
                    device.rustdesk_id.as_deref(),
                    &server_config,
                )
                .unwrap_or_default(),
                hostname_display: device.hostname.clone().unwrap_or_else(|| "-".to_string()),
                last_checkin_display: format_last_checkin_display(
                    device.last_checkin_at.as_deref(),
                ),
                recently_seen: device_recently_seen,
                status_display: if device_recently_seen {
                    "Recently seen".to_string()
                } else {
                    "Not recently seen".to_string()
                },
                archived_display: if device.archived {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
            }
        })
        .collect();
    let view = DevicesListView {
        title: "Devices".to_string(),
        show_nav: true,
        nav: nav_permissions_for_role(user.parsed_role()),
        csrf_token: user.csrf_token.clone(),
        search_term: search_term.clone(),
        export_csv_href: export_csv_href(&search_term, &user.csrf_token),
        archived_hide: archived_filter == "0",
        archived_only: archived_filter == "1",
        archived_all: archived_filter == "all",
        seen_recent: seen_filter == "recent",
        empty_inventory,
        filter_empty,
        devices: rows,
    };
    let html = view
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Html(html).into_response())
}
