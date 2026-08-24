use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use time::OffsetDateTime;

use crate::app_state::AppState;
use crate::repository::client_access_tokens::authorize_client_token_for_device;

use super::client_account::json_error;

const INTERNAL_AUTH_HEADER: &str = "x-opendesk-transport-key";
const INVALID_REQUEST: &str = "Invalid request";

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/internal/transport/authorize",
        post(authorize_transport_request),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransportAuthorizationRequest {
    token: String,
    rustdesk_id: String,
}

#[derive(Serialize)]
struct TransportAuthorizationResponse {
    allowed: bool,
}

async fn authorize_transport_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TransportAuthorizationRequest>,
) -> Result<Json<TransportAuthorizationResponse>, (StatusCode, Json<Value>)> {
    authenticate_transport(&state, &headers)?;
    if !valid_field(&request.token, 128) || !valid_field(&request.rustdesk_id, 64) {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    }
    let allowed = authorize_client_token_for_device(
        &state.db,
        &state.client_token_hmac_key,
        &request.token,
        &request.rustdesk_id,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(|_| json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"))?;
    Ok(Json(TransportAuthorizationResponse { allowed }))
}

fn authenticate_transport(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<Value>)> {
    let configured = state
        .transport_introspection_key
        .as_deref()
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Not found"))?;
    let supplied = headers
        .get(INTERNAL_AUTH_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| hex::decode(value).ok())
        .ok_or_else(|| json_error(StatusCode::UNAUTHORIZED, "Unauthorized"))?;
    let mut verifier = Hmac::<Sha256>::new_from_slice(configured)
        .map_err(|_| json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"))?;
    verifier.update(b"opendesk-transport-introspection-v1");
    if verifier.verify_slice(&supplied).is_err() {
        return Err(json_error(StatusCode::UNAUTHORIZED, "Unauthorized"));
    }
    Ok(())
}

fn valid_field(value: &str, maximum_length: usize) -> bool {
    !value.is_empty() && value.len() <= maximum_length && !value.chars().any(char::is_control)
}
