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
use crate::domain::address_book::normalize_optional_notes;
use crate::domain::role::Role;
use crate::http::session::{require_action, require_csrf};
use crate::http::views::{
    nav_permissions_for_role, AddressBookDetailView, AddressBookDeviceOptionView,
    AddressBookEntryRowView, AddressBookRowView, AddressBooksListView,
};
use crate::repository::address_books::{
    create_address_book, create_address_book_entry, delete_address_book_entry,
    find_address_book_entry, find_address_book_for_owner, list_address_book_entries,
    list_address_books_for_owner, update_address_book_entry, AddressBookRepositoryError,
};
use crate::repository::device_visibility::is_device_visible_to_user;
use crate::repository::device_visibility::list_visible_device_uuids_for_user;
use crate::repository::devices::list_devices;

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
            axum::routing::post(address_book_entry_create_submit),
        )
        .route(
            "/address-books/{address_book_uuid}/entries/{address_book_entry_uuid}",
            axum::routing::post(address_book_entry_update_submit),
        )
        .route(
            "/address-books/{address_book_uuid}/entries/{address_book_entry_uuid}/delete",
            axum::routing::post(address_book_entry_delete_submit),
        )
}

#[derive(Deserialize)]
struct AddressBookCreateForm {
    csrf_token: String,
    name: String,
}

#[derive(Deserialize)]
struct AddressBookEntryForm {
    #[serde(default)]
    csrf_token: String,
    device_uuid: String,
    alias: String,
    notes: Option<String>,
    position: String,
}

fn parse_entry(form: &AddressBookEntryForm) -> Option<(Uuid, String, Option<String>, u32)> {
    Some((
        Uuid::parse_str(form.device_uuid.trim()).ok()?,
        form.alias.clone(),
        normalize_optional_notes(form.notes.clone()),
        form.position.trim().parse().ok()?,
    ))
}

fn repository_error_response(error: AddressBookRepositoryError) -> Response {
    match error {
        AddressBookRepositoryError::NotFound => StatusCode::NOT_FOUND.into_response(),
        AddressBookRepositoryError::Validation(_) | AddressBookRepositoryError::Conflict => {
            StatusCode::BAD_REQUEST.into_response()
        }
        AddressBookRepositoryError::Database(_) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn require_visible_device_or_not_found(
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
    let can_create = Action::AddressBookCreate.allowed_for(role);
    let can_update = Action::AddressBookUpdate.allowed_for(role);
    let can_delete = Action::AddressBookDelete.allowed_for(role);
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
                address_book_uuid: book.address_book_uuid.to_string(),
                name: book.name,
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

async fn address_book_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<AddressBookCreateForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookCreate).await?;
    require_csrf(&actor, &form.csrf_token)?;
    let book = create_address_book(&state.db, actor.user_uuid, form.name.trim())
        .await
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
    let can_create = Action::AddressBookCreate.allowed_for(role);
    let can_update = Action::AddressBookUpdate.allowed_for(role);
    let can_delete = Action::AddressBookDelete.allowed_for(role);
    let entries = list_address_book_entries(&state.db, owner_user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    let visible = list_visible_device_uuids_for_user(&state.db, owner_user_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let visible: std::collections::HashSet<Uuid> = visible.into_iter().collect();
    let devices = list_devices(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
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
        entries: entries
            .into_iter()
            .map(|entry| AddressBookEntryRowView {
                address_book_entry_uuid: entry.address_book_entry_uuid.to_string(),
                device_uuid: entry.device_uuid.to_string(),
                alias: entry.alias,
                notes: entry.notes.unwrap_or_default(),
                position: entry.position,
            })
            .collect(),
        device_options: devices
            .into_iter()
            .filter(|device| visible.contains(&device.device_uuid))
            .map(|device| AddressBookDeviceOptionView {
                device_uuid: device.device_uuid.to_string(),
                alias: device.alias,
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

async fn address_book_entry_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(address_book_uuid): Path<Uuid>,
    Form(form): Form<AddressBookEntryForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookUpdate).await?;
    let _book = find_address_book_for_owner(&state.db, actor.user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    let Some((device_uuid, alias, notes, position)) = parse_entry(&form) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };
    require_visible_device_or_not_found(&state, actor.user_uuid, device_uuid).await?;
    require_csrf(&actor, &form.csrf_token)?;
    create_address_book_entry(
        &state.db,
        actor.user_uuid,
        address_book_uuid,
        device_uuid,
        &alias,
        notes,
        position,
    )
    .await
    .map_err(repository_error_response)?;
    Ok(Redirect::to(&format!("/address-books/{address_book_uuid}")).into_response())
}

async fn address_book_entry_update_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((address_book_uuid, address_book_entry_uuid)): Path<(Uuid, Uuid)>,
    Form(form): Form<AddressBookEntryForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookUpdate).await?;
    let _book = find_address_book_for_owner(&state.db, actor.user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    let Some((device_uuid, alias, notes, position)) = parse_entry(&form) else {
        return Err(StatusCode::BAD_REQUEST.into_response());
    };
    let entry = find_address_book_entry(&state.db, actor.user_uuid, address_book_entry_uuid)
        .await
        .map_err(repository_error_response)?;
    if entry.address_book_uuid != address_book_uuid {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    require_visible_device_or_not_found(&state, actor.user_uuid, device_uuid).await?;
    require_csrf(&actor, &form.csrf_token)?;
    let updated = update_address_book_entry(
        &state.db,
        actor.user_uuid,
        address_book_entry_uuid,
        device_uuid,
        &alias,
        notes,
        position,
    )
    .await
    .map_err(repository_error_response)?;
    if updated.address_book_uuid != address_book_uuid {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    Ok(Redirect::to(&format!("/address-books/{address_book_uuid}")).into_response())
}

async fn address_book_entry_delete_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Path((address_book_uuid, address_book_entry_uuid)): Path<(Uuid, Uuid)>,
    Form(form): Form<DeleteEntryForm>,
) -> Result<Response, Response> {
    let actor = require_action(&state, &jar, Action::AddressBookDelete).await?;
    let _book = find_address_book_for_owner(&state.db, actor.user_uuid, address_book_uuid)
        .await
        .map_err(repository_error_response)?;
    let entry = find_address_book_entry(&state.db, actor.user_uuid, address_book_entry_uuid)
        .await
        .map_err(repository_error_response)?;
    if entry.address_book_uuid != address_book_uuid {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    require_visible_device_or_not_found(&state, actor.user_uuid, entry.device_uuid).await?;
    require_csrf(&actor, &form.csrf_token)?;
    delete_address_book_entry(&state.db, actor.user_uuid, address_book_entry_uuid)
        .await
        .map_err(repository_error_response)?;
    Ok(Redirect::to(&format!("/address-books/{address_book_uuid}")).into_response())
}

#[derive(Deserialize)]
struct DeleteEntryForm {
    #[serde(default)]
    csrf_token: String,
}
