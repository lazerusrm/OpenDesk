use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::repository::address_books::{
    find_personal_address_book, list_address_book_tags_for_client, AddressBookRepositoryError,
};

use super::client_sync::{authenticate, client_address_book_peers};

/// Released RustDesk 1.4.9 legacy fallback boundary. The client calls this
/// after `POST /api/ab/personal` returns 404 and expects nested JSON text.
pub fn routes() -> Router<AppState> {
    Router::new().route("/api/ab", get(legacy_address_book))
}

async fn legacy_address_book(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user_uuid = authenticate(&state, &headers).await?;
    let book = match find_personal_address_book(&state.db, user_uuid).await {
        Ok(book) => book,
        Err(AddressBookRepositoryError::NotFound) => return Ok(Json(Value::Null)),
        Err(error) => return Err(internal_error(error)),
    };
    let peers = client_address_book_peers(&state, user_uuid, book.address_book_uuid).await?;
    let tags = list_address_book_tags_for_client(&state.db, user_uuid, book.address_book_uuid)
        .await
        .map_err(internal_error)?;
    let tag_names: Vec<String> = tags.iter().map(|(name, _)| name.clone()).collect();
    let tag_colors: serde_json::Map<String, Value> = tags
        .into_iter()
        .map(|(name, color)| (name, json!(color)))
        .collect();
    let data = json!({
        "tags": tag_names,
        "peers": peers,
        "tag_colors": Value::Object(tag_colors).to_string(),
    });
    Ok(Json(json!({
        "data": data.to_string(),
        "licensed_devices": 10000,
    })))
}

fn internal_error(_: impl std::fmt::Debug) -> (StatusCode, Json<Value>) {
    super::client_account::json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}
