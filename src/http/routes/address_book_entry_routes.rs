use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use super::address_book_forms::{parse_entry, AddressBookEntryForm};
use super::{repository_error_response, require_visible_device_or_not_found};
use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::http::session::{require_action, require_csrf};
use crate::repository::address_books::{
    create_address_book_entry, delete_address_book_entry, find_address_book_entry,
    find_address_book_for_owner, update_address_book_entry,
};

#[derive(Deserialize)]
pub(super) struct DeleteEntryForm {
    #[serde(default)]
    pub csrf_token: String,
}

pub(super) async fn address_book_entry_create_submit(
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

pub(super) async fn address_book_entry_update_submit(
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

pub(super) async fn address_book_entry_delete_submit(
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
    crate::repository::address_books::hide_personal_address_book_device(
        &state.db,
        actor.user_uuid,
        address_book_uuid,
        entry.device_uuid,
    )
    .await
    .map_err(repository_error_response)?;
    delete_address_book_entry(&state.db, actor.user_uuid, address_book_entry_uuid)
        .await
        .map_err(repository_error_response)?;
    Ok(Redirect::to(&format!("/address-books/{address_book_uuid}")).into_response())
}
