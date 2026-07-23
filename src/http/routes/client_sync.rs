use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::repository::address_books::{
    find_address_book_for_client_access, find_personal_address_book,
    list_address_book_tags_for_client, list_shared_address_books_for_user,
};
use crate::repository::{
    client_access_tokens::authenticate_client_bearer_token,
    client_sync::{
        list_client_access_groups, list_client_accessible_users, list_client_address_book_peers,
        list_client_visible_peers,
    },
};

use super::client_account::{bearer_token, json_error};

const INVALID_REQUEST: &str = "Invalid request";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/device-group/accessible", get(device_groups))
        .route("/api/users", get(users))
        .route("/api/peers", get(peers))
        .route("/api/ab/settings", post(address_book_settings))
        .route("/api/ab/personal", post(personal_address_book))
        .route("/api/ab/shared/profiles", post(shared_address_books))
        .route("/api/ab/peers", post(address_book_peers))
        .route("/api/ab/tags/{guid}", post(address_book_tags))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    current: usize,
    #[serde(rename = "pageSize")]
    page_size: usize,
    accessible: Option<String>,
    status: Option<String>,
}

#[derive(Serialize)]
struct PageResponse<T> {
    total: usize,
    data: Vec<T>,
}

#[derive(Serialize)]
struct DeviceGroupResponse {
    name: String,
}

#[derive(Serialize)]
struct UserResponse {
    name: String,
    display_name: String,
    avatar: String,
    email: String,
    note: String,
    status: i32,
    is_admin: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddressBookPageQuery {
    current: usize,
    #[serde(rename = "pageSize")]
    page_size: usize,
    ab: String,
}

#[derive(Serialize)]
struct AddressBookProfileResponse {
    guid: String,
    name: String,
    owner: String,
    note: String,
    rule: i32,
    info: Value,
}

#[derive(Serialize)]
struct AddressBookTagResponse {
    name: String,
    color: i64,
}

#[derive(Serialize)]
pub(super) struct AddressBookPeerResponse {
    id: String,
    hash: String,
    password: String,
    username: String,
    hostname: String,
    platform: String,
    alias: String,
    tags: Vec<String>,
    #[serde(rename = "forceAlwaysRelay")]
    force_always_relay: String,
    #[serde(rename = "rdpPort")]
    rdp_port: String,
    #[serde(rename = "rdpUsername")]
    rdp_username: String,
    #[serde(rename = "loginName")]
    login_name: String,
    device_group_name: String,
    note: String,
    same_server: bool,
}

#[derive(Serialize)]
struct PeerResponse {
    id: String,
    info: Value,
    status: i32,
    user: String,
    user_name: String,
    device_group_name: String,
    note: String,
}

async fn device_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Result<Json<PageResponse<DeviceGroupResponse>>, (StatusCode, Json<Value>)> {
    validate_page(&query, false)?;
    let user_uuid = authenticate(&state, &headers).await?;
    let groups = list_client_access_groups(&state.db, user_uuid)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|name| DeviceGroupResponse { name })
        .collect();
    Ok(Json(page(groups, &query)))
}

async fn users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Result<Json<PageResponse<UserResponse>>, (StatusCode, Json<Value>)> {
    validate_page(&query, true)?;
    let user_uuid = authenticate(&state, &headers).await?;
    let users = list_client_accessible_users(&state.db, user_uuid)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|(_, username, role)| UserResponse {
            display_name: username.clone(),
            name: username,
            avatar: String::new(),
            email: String::new(),
            note: String::new(),
            status: 1,
            is_admin: role == "admin",
        })
        .collect();
    Ok(Json(page(users, &query)))
}

async fn peers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Result<Json<PageResponse<PeerResponse>>, (StatusCode, Json<Value>)> {
    validate_page(&query, true)?;
    let user_uuid = authenticate(&state, &headers).await?;
    let peers = list_client_visible_peers(&state.db, user_uuid)
        .await
        .map_err(internal_error)?
        .into_iter()
        .map(|peer| {
            let os = match (&peer.os_family, &peer.os_version) {
                (Some(family), Some(version)) => format!("{family} / {version}"),
                (Some(family), None) => family.clone(),
                _ => String::new(),
            };
            PeerResponse {
                id: peer.rustdesk_id,
                info: json!({
                    "username": peer.owner.unwrap_or_default(),
                    "device_name": peer.hostname.unwrap_or(peer.alias),
                    "os": os,
                }),
                status: 1,
                user: String::new(),
                user_name: String::new(),
                device_group_name: String::new(),
                note: peer.notes.unwrap_or_default(),
            }
        })
        .collect();
    Ok(Json(page(peers, &query)))
}

async fn address_book_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    authenticate(&state, &headers).await?;
    Ok(Json(json!({ "max_peer_one_ab": 10000 })))
}

async fn personal_address_book(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_uuid = authenticate(&state, &headers).await?;
    let book = find_personal_address_book(&state.db, user_uuid)
        .await
        .map_err(|_| json_error(StatusCode::NOT_FOUND, "Address book not found"))?;
    Ok(Json(json!({ "guid": book.address_book_uuid.to_string() })))
}

async fn shared_address_books(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Result<Json<PageResponse<AddressBookProfileResponse>>, (StatusCode, Json<Value>)> {
    validate_page(&query, false)?;
    let user_uuid = authenticate(&state, &headers).await?;
    let books = list_shared_address_books_for_user(&state.db, user_uuid)
        .await
        .map_err(internal_error)?;
    let profiles = books
        .into_iter()
        .map(|(book, owner, permission)| AddressBookProfileResponse {
            guid: book.address_book_uuid.to_string(),
            name: book.name,
            owner,
            note: String::new(),
            rule: match permission.as_str() {
                "read" => 1,
                "write" => 2,
                "admin" => 3,
                _ => 0,
            },
            info: json!({}),
        })
        .collect();
    Ok(Json(page(profiles, &query)))
}

async fn address_book_peers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AddressBookPageQuery>,
) -> Result<Json<PageResponse<AddressBookPeerResponse>>, (StatusCode, Json<Value>)> {
    validate_address_book_page(&query)?;
    let user_uuid = authenticate(&state, &headers).await?;
    let book_uuid = parse_guid(&query.ab)?;
    find_address_book_for_client_access(&state.db, user_uuid, book_uuid)
        .await
        .map_err(|_| json_error(StatusCode::NOT_FOUND, "Address book not found"))?;
    let peers = client_address_book_peers(&state, user_uuid, book_uuid).await?;
    let page_query = PageQuery {
        current: query.current,
        page_size: query.page_size,
        accessible: None,
        status: None,
    };
    Ok(Json(page(peers, &page_query)))
}

pub(super) async fn client_address_book_peers(
    state: &AppState,
    user_uuid: uuid::Uuid,
    book_uuid: uuid::Uuid,
) -> Result<Vec<AddressBookPeerResponse>, (StatusCode, Json<Value>)> {
    Ok(
        list_client_address_book_peers(&state.db, user_uuid, book_uuid)
            .await
            .map_err(internal_error)?
            .into_iter()
            .map(|peer| AddressBookPeerResponse {
                id: peer.rustdesk_id,
                hash: String::new(),
                password: String::new(),
                username: peer.owner.unwrap_or_default(),
                hostname: peer.hostname.unwrap_or_default(),
                platform: peer.os_family.unwrap_or_default(),
                alias: peer.alias,
                tags: peer.tags,
                force_always_relay: "false".to_string(),
                rdp_port: String::new(),
                rdp_username: String::new(),
                login_name: String::new(),
                device_group_name: String::new(),
                note: peer.notes.unwrap_or_default(),
                same_server: true,
            })
            .collect(),
    )
}

async fn address_book_tags(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(guid): Path<String>,
) -> Result<Json<Vec<AddressBookTagResponse>>, (StatusCode, Json<Value>)> {
    let user_uuid = authenticate(&state, &headers).await?;
    let book_uuid = parse_guid(&guid)?;
    let tags = list_address_book_tags_for_client(&state.db, user_uuid, book_uuid)
        .await
        .map_err(|error| match error {
            crate::repository::address_books::AddressBookRepositoryError::NotFound => {
                json_error(StatusCode::NOT_FOUND, "Address book not found")
            }
            _ => internal_error(error),
        })?
        .into_iter()
        .map(|(name, color)| AddressBookTagResponse { name, color })
        .collect();
    Ok(Json(tags))
}

pub(super) async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<uuid::Uuid, (StatusCode, Json<Value>)> {
    let value = bearer_token(headers)
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "Unauthorized"))?;
    authenticate_client_bearer_token(
        &state.db,
        &state.client_token_hmac_key,
        value,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(internal_error)?
    .map(|token| token.user_uuid)
    .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "Unauthorized"))
}

fn validate_page(
    query: &PageQuery,
    requires_filters: bool,
) -> Result<(), (StatusCode, Json<Value>)> {
    if query.current == 0
        || query.page_size == 0
        || query.page_size > 100
        || (requires_filters
            && (query.accessible.as_deref() != Some("") || query.status.as_deref() != Some("1")))
        || (!requires_filters && (query.accessible.is_some() || query.status.is_some()))
    {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    }
    Ok(())
}

fn validate_address_book_page(
    query: &AddressBookPageQuery,
) -> Result<(), (StatusCode, Json<Value>)> {
    if query.current == 0 || query.page_size == 0 || query.page_size > 100 {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    }
    parse_guid(&query.ab)?;
    Ok(())
}

pub(super) fn parse_guid(value: &str) -> Result<uuid::Uuid, (StatusCode, Json<Value>)> {
    uuid::Uuid::parse_str(value).map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))
}

fn page<T>(items: Vec<T>, query: &PageQuery) -> PageResponse<T> {
    let total = items.len();
    let start = (query.current - 1).saturating_mul(query.page_size);
    let data = items
        .into_iter()
        .skip(start)
        .take(query.page_size)
        .collect();
    PageResponse { total, data }
}

fn internal_error(_: impl std::fmt::Debug) -> (StatusCode, Json<Value>) {
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}
