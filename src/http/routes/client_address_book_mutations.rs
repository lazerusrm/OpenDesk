use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::app_state::AppState;
use crate::repository::address_books::AddressBookRepositoryError;
use crate::repository::client_address_book_mutations::{
    add_client_address_book_peer, add_client_address_book_tag, delete_client_address_book_peers,
    delete_client_address_book_tags, rename_client_address_book_tag,
    update_client_address_book_peer, update_client_address_book_tag_color,
    ClientAddressBookPeerDraft, ClientAddressBookPeerUpdate,
};

use crate::domain::role::Role;
use crate::repository::users::find_user_by_uuid;

use super::client_account::json_error;
use super::client_sync::{authenticate, parse_guid};

const INVALID_REQUEST: &str = "Invalid request";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ab/peer/add/{guid}", post(add_peer))
        .route("/api/ab/peer/update/{guid}", put(update_peer))
        .route("/api/ab/peer/{guid}", delete(delete_peers))
        .route("/api/ab/tag/add/{guid}", post(add_tag))
        .route("/api/ab/tag/rename/{guid}", put(rename_tag))
        .route("/api/ab/tag/update/{guid}", put(update_tag))
        .route("/api/ab/tag/{guid}", delete(delete_tags))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddPeerRequest {
    id: String,
    #[serde(default)]
    alias: String,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    password: Option<String>,
    hash: Option<String>,
    username: Option<String>,
    hostname: Option<String>,
    platform: Option<String>,
    #[serde(rename = "forceAlwaysRelay")]
    force_always_relay: Option<String>,
    #[serde(rename = "rdpPort")]
    rdp_port: Option<String>,
    #[serde(rename = "rdpUsername")]
    rdp_username: Option<String>,
    #[serde(rename = "loginName")]
    login_name: Option<String>,
    device_group_name: Option<String>,
    same_server: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdatePeerRequest {
    id: String,
    alias: Option<String>,
    note: Option<String>,
    tags: Option<Vec<String>>,
    password: Option<String>,
    hash: Option<String>,
    username: Option<String>,
    hostname: Option<String>,
    platform: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TagRequest {
    name: String,
    color: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameTagRequest {
    old: String,
    new: String,
}

async fn add_peer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(request): Json<AddPeerRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let _ignored_secrets = (request.password, request.hash);
    let _ignored_client_metadata = (
        request.username,
        request.hostname,
        request.platform,
        request.force_always_relay,
        request.rdp_port,
        request.rdp_username,
        request.login_name,
        request.device_group_name,
        request.same_server,
    );
    let user_uuid = authenticate(&state, &headers).await?;
    let alias = if request.alias.trim().is_empty() {
        request.id.clone()
    } else {
        request.alias
    };
    add_client_address_book_peer(
        &state.db,
        user_uuid,
        parse_guid(&guid)?,
        ClientAddressBookPeerDraft {
            rustdesk_id: request.id,
            alias,
            notes: request.note,
            tags: request.tags,
        },
    )
    .await
    .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn update_peer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(request): Json<UpdatePeerRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    // Official Flutter PUTs `{id, hash}` after a successful session and toasts
    // any JSON `error`. Acknowledge without persisting those secrets.
    let _ignored_secrets = (request.password, request.hash);
    let _ignored_client_metadata = (request.username, request.hostname, request.platform);
    let user_uuid = authenticate(&state, &headers).await?;
    if request.alias.is_none() && request.note.is_none() && request.tags.is_none() {
        return Ok(StatusCode::OK);
    }
    update_client_address_book_peer(
        &state.db,
        user_uuid,
        parse_guid(&guid)?,
        ClientAddressBookPeerUpdate {
            rustdesk_id: request.id,
            alias: request.alias,
            notes: request.note.map(Some),
            tags: request.tags,
        },
    )
    .await
    .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn delete_peers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(ids): Json<Vec<String>>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if ids.is_empty() {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    }
    let user_uuid = authenticate(&state, &headers).await?;
    require_admin_user(&state, user_uuid).await?;
    delete_client_address_book_peers(&state.db, user_uuid, parse_guid(&guid)?, &ids)
        .await
        .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn add_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(request): Json<TagRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let user_uuid = authenticate(&state, &headers).await?;
    add_client_address_book_tag(
        &state.db,
        user_uuid,
        parse_guid(&guid)?,
        &request.name,
        request.color,
    )
    .await
    .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn rename_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(request): Json<RenameTagRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let user_uuid = authenticate(&state, &headers).await?;
    rename_client_address_book_tag(
        &state.db,
        user_uuid,
        parse_guid(&guid)?,
        &request.old,
        &request.new,
    )
    .await
    .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn update_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(request): Json<TagRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let user_uuid = authenticate(&state, &headers).await?;
    update_client_address_book_tag_color(
        &state.db,
        user_uuid,
        parse_guid(&guid)?,
        &request.name,
        request.color,
    )
    .await
    .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn delete_tags(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
    Json(names): Json<Vec<String>>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    if names.is_empty() {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    }
    let user_uuid = authenticate(&state, &headers).await?;
    require_admin_user(&state, user_uuid).await?;
    delete_client_address_book_tags(&state.db, user_uuid, parse_guid(&guid)?, &names)
        .await
        .map_err(repository_error)?;
    Ok(StatusCode::OK)
}

async fn require_admin_user(
    state: &AppState,
    user_uuid: uuid::Uuid,
) -> Result<(), (StatusCode, Json<Value>)> {
    let user = find_user_by_uuid(&state.db, user_uuid)
        .await
        .map_err(|_| json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"))?
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "Unauthorized"))?;
    if user.role != Role::ADMIN {
        return Err(json_error(StatusCode::FORBIDDEN, "Forbidden"));
    }
    Ok(())
}

fn repository_error(error: AddressBookRepositoryError) -> (StatusCode, Json<Value>) {
    match error {
        AddressBookRepositoryError::Forbidden => json_error(StatusCode::FORBIDDEN, "Forbidden"),
        AddressBookRepositoryError::NotFound => {
            json_error(StatusCode::NOT_FOUND, "Address book or peer not found")
        }
        AddressBookRepositoryError::Conflict | AddressBookRepositoryError::Validation(_) => {
            json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST)
        }
        AddressBookRepositoryError::Database(_) => {
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}
