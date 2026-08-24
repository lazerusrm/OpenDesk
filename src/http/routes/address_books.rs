use std::collections::{HashMap, HashSet};

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::address_book::AddressBookEntry;
use crate::domain::device::Device;
use crate::domain::device_list::{format_notes_display, notes_list_title, rustdesk_id_copy_text};
use crate::domain::role::Role;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{
    nav_permissions_for_role, AddressBookDetailView, AddressBookDeviceOptionView,
    AddressBookEntryRowView, AddressBookRowView, AddressBooksListView,
};
use crate::repository::address_books::{
    create_personal_address_book, create_shared_address_book, find_address_book_for_owner,
    find_personal_address_book, list_address_book_entries, list_address_books_for_owner,
    AddressBookRepositoryError,
};
use crate::repository::device_visibility::is_device_visible_to_user;
use crate::repository::device_visibility::list_visible_device_uuids_for_user;
use crate::repository::devices::list_devices;

#[path = "address_book_entry_routes.rs"]
mod address_book_entry_routes;
#[path = "address_book_forms.rs"]
mod address_book_forms;
#[path = "address_book_sharing.rs"]
mod address_book_sharing;
use address_book_forms::AddressBookCreateForm;
use address_book_sharing::{load_owned_book_share_view, load_shared_with_me};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/address-books",
            get(address_books_list).post(address_book_create_submit),
        )
        .route(
            "/address-books/{address_book_uuid}",
            get(address_book_detail),
        )
        .route(
            "/address-books/{address_book_uuid}/entries",
            axum::routing::post(address_book_entry_routes::address_book_entry_create_submit),
        )
        .route(
            "/address-books/{address_book_uuid}/entries/{address_book_entry_uuid}",
            axum::routing::post(address_book_entry_routes::address_book_entry_update_submit),
        )
        .route(
            "/address-books/{address_book_uuid}/entries/{address_book_entry_uuid}/delete",
            axum::routing::post(address_book_entry_routes::address_book_entry_delete_submit),
        )
        .route(
            "/address-books/{address_book_uuid}/access-rules",
            axum::routing::post(address_book_sharing::address_book_access_rule_create_submit),
        )
        .route(
            "/address-books/{address_book_uuid}/access-rules/{principal_type}/{principal_uuid}/delete",
            axum::routing::post(address_book_sharing::address_book_access_rule_delete_submit),
        )
}

pub(super) fn repository_error_response(error: AddressBookRepositoryError) -> Response {
    match error {
        AddressBookRepositoryError::Forbidden => StatusCode::FORBIDDEN.into_response(),
        AddressBookRepositoryError::NotFound => StatusCode::NOT_FOUND.into_response(),
        AddressBookRepositoryError::Validation(_) | AddressBookRepositoryError::Conflict => {
            StatusCode::BAD_REQUEST.into_response()
        }
        AddressBookRepositoryError::Database(_) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub(super) async fn require_visible_device_or_not_found(
    state: &AppState,
    owner_user_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<(), Response> {
    let visible = is_device_visible_to_user(&state.db, owner_user_uuid, device_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    if visible {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND.into_response())
    }
}

async fn address_books_list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookList).await?;
    render_books_list(
        &state,
        actor.user_uuid,
        &actor.csrf_token,
        nav_permissions_for_role(actor.parsed_role()),
        actor.parsed_role(),
        None,
    )
    .await
}

async fn render_books_list(
    state: &AppState,
    owner_user_uuid: Uuid,
    csrf_token: &str,
    nav: crate::http::views::NavPermissions,
    role: Role,
    error_message: Option<String>,
) -> Result<Response, Response> {
    let books = list_address_books_for_owner(&state.db, owner_user_uuid)
        .await
        .map_err(repository_error_response)?;
    let personal_uuid = personal_address_book_uuid(state, owner_user_uuid).await?;
    let can_create = Action::AddressBookCreate.allowed_for(role);
    let can_update = Action::AddressBookUpdate.allowed_for(role);
    let can_delete = Action::AddressBookDelete.allowed_for(role);
    let shared_books = load_shared_with_me(state, owner_user_uuid).await?;
    let view = AddressBooksListView {
        title: "Address books".to_string(),
        show_nav: true,
        nav,
        can_create,
        can_update,
        can_delete,
        csrf_token: csrf_token.to_string(),
        books: books
            .into_iter()
            .map(|book| AddressBookRowView {
                kind_display: book_kind_display(book.address_book_uuid, personal_uuid),
                address_book_uuid: book.address_book_uuid.to_string(),
                name: book.name,
            })
            .collect(),
        shared_books,
        error_message,
    };
    Ok(Html(
        view.render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?,
    )
    .into_response())
}

async fn address_book_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<AddressBookCreateForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookCreate).await?;
    require_csrf(&actor, &form.csrf_token)?;
    let book = match form.book_kind.as_str() {
        "personal" => {
            create_personal_address_book(&state.db, actor.user_uuid, form.name.trim()).await
        }
        "shared" => create_shared_address_book(&state.db, actor.user_uuid, form.name.trim()).await,
        _ => return Err(StatusCode::BAD_REQUEST.into_response()),
    }
    .map_err(repository_error_response)?;
    Ok(Redirect::to(&format!("/address-books/{}", book.address_book_uuid)).into_response())
}

async fn address_book_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(address_book_uuid): Path<Uuid>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookView).await?;
    render_book_detail(
        &state,
        actor.user_uuid,
        address_book_uuid,
        &actor.csrf_token,
        nav_permissions_for_role(actor.parsed_role()),
        actor.parsed_role(),
        None,
    )
    .await
}

async fn render_book_detail(
    state: &AppState,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    csrf_token: &str,
    nav: crate::http::views::NavPermissions,
    role: Role,
    error_message: Option<String>,
) -> Result<Response, Response> {
    let book = find_address_book_for_owner(&state.db, owner_user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    let personal_uuid = personal_address_book_uuid(state, owner_user_uuid).await?;
    let can_create = Action::AddressBookCreate.allowed_for(role);
    let can_update = Action::AddressBookUpdate.allowed_for(role);
    let can_delete = Action::AddressBookDelete.allowed_for(role);
    let entries = list_address_book_entries(&state.db, owner_user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    let visible = list_visible_device_uuids_for_user(&state.db, owner_user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let visible: HashSet<Uuid> = visible.into_iter().collect();
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let visible_devices: Vec<Device> = devices
        .into_iter()
        .filter(|device| visible.contains(&device.device_uuid))
        .collect();
    let device_by_uuid: HashMap<Uuid, &Device> = visible_devices
        .iter()
        .map(|device| (device.device_uuid, device))
        .collect();
    let used_devices: HashSet<Uuid> = entries.iter().map(|entry| entry.device_uuid).collect();
    let kind_display = book_kind_display(book.address_book_uuid, personal_uuid);
    let share = load_owned_book_share_view(
        state,
        owner_user_uuid,
        address_book_uuid,
        &kind_display,
        can_update,
    )
    .await?;
    let view = AddressBookDetailView {
        title: format!("Address book · {}", book.name),
        show_nav: true,
        nav,
        can_create,
        can_update,
        can_delete,
        csrf_token: csrf_token.to_string(),
        address_book_uuid: book.address_book_uuid.to_string(),
        name: book.name,
        kind_display,
        can_share: share.can_share,
        access_rules: share.access_rules,
        user_share_options: share.user_share_options,
        group_share_options: share.group_share_options,
        entries: entries
            .into_iter()
            .map(|entry| {
                let device = device_by_uuid.get(&entry.device_uuid).copied();
                entry_row_view(entry, device)
            })
            .collect(),
        device_options: visible_devices
            .iter()
            .filter(|device| !used_devices.contains(&device.device_uuid))
            .map(|device| AddressBookDeviceOptionView {
                device_uuid: device.device_uuid.to_string(),
                alias: device.alias.clone(),
                rustdesk_id: rustdesk_id_copy_text(device.rustdesk_id.as_deref())
                    .unwrap_or_default(),
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

fn book_kind_display(address_book_uuid: Uuid, personal_uuid: Option<Uuid>) -> String {
    if personal_uuid == Some(address_book_uuid) {
        "Personal".to_string()
    } else {
        "Shared".to_string()
    }
}

fn entry_row_view(entry: AddressBookEntry, device: Option<&Device>) -> AddressBookEntryRowView {
    let rustdesk_id = device.and_then(|device| device.rustdesk_id.clone());
    AddressBookEntryRowView {
        address_book_entry_uuid: entry.address_book_entry_uuid.to_string(),
        device_uuid: entry.device_uuid.to_string(),
        device_alias: device
            .map(|device| device.alias.clone())
            .unwrap_or_else(|| "-".to_string()),
        alias: entry.alias,
        notes: entry.notes.clone().unwrap_or_default(),
        notes_display: format_notes_display(entry.notes.as_deref()),
        notes_title: notes_list_title(entry.notes.as_deref()),
        position: entry.position,
        rustdesk_id_display: rustdesk_id.clone().unwrap_or_else(|| "-".to_string()),
        rustdesk_id_copy_text: rustdesk_id_copy_text(rustdesk_id.as_deref()).unwrap_or_default(),
    }
}

async fn personal_address_book_uuid(
    state: &AppState,
    owner_user_uuid: Uuid,
) -> Result<Option<Uuid>, Response> {
    match find_personal_address_book(&state.db, owner_user_uuid).await {
        Ok(book) => Ok(Some(book.address_book_uuid)),
        Err(AddressBookRepositoryError::NotFound) => Ok(None),
        Err(error) => Err(repository_error_response(error)),
    }
}
