use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::app_state::AppState;

use super::client_account::json_error;
use super::client_sync::authenticate;

const INVALID_REQUEST: &str = "Invalid request";

/// Isolated `/api/ab/password` boundary.
/// Official clients send `{id, password}` or `{id, hash}` and toast a JSON
/// `error` field. Authenticate like other `/api/ab*` routes, ignore those
/// secrets, and acknowledge with empty 200 so the session UI stays quiet.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/ab/password",
            post(address_book_password).put(address_book_password),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddressBookPasswordRequest {
    id: Option<String>,
    guid: Option<String>,
    password: Option<String>,
    hash: Option<String>,
}

async fn address_book_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    authenticate(&state, &headers).await?;
    if !body.is_empty() {
        let _request: AddressBookPasswordRequest = serde_json::from_slice(&body)
            .map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))?;
        let _ignored_client_metadata = (_request.id, _request.guid);
        let _ignored_secrets = (_request.password, _request.hash);
    }
    Ok(StatusCode::OK)
}
