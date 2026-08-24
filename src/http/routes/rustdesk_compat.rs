use std::collections::HashMap;

use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app_state::AppState;

#[path = "rustdesk_compat_persist.rs"]
mod rustdesk_compat_persist;
use rustdesk_compat_persist::{
    persist_connection_audit, persist_file_transfer, persist_last_seen, persist_note,
    persist_session_record, persist_sysinfo,
};

const MAX_ID_LENGTH: usize = 64;
const MAX_UUID_LENGTH: usize = 128;
const MAX_VERSION_LENGTH: usize = 128;
const MAX_SYSINFO_FIELDS: usize = 64;
const MAX_SYSINFO_DEPTH: usize = 4;
const MAX_TEXT_LENGTH: usize = 1024;
const MAX_ARRAY_LENGTH: usize = 128;
const MAX_AUDIT_NOTE_LENGTH: usize = 1024;
const BOUNDED_JSON_BYTES: usize = 16 * 1024;
const RECORD_TYPES: &[&str] = &["new", "part", "tail", "remove"];

/// RustDesk-shaped compatibility boundary for official-client liveness posts.
/// Existing rustdesk_id values may refresh last_checkin_at; unknown ids are not enrolled.
/// Connection-audit rows are launch/liveness evidence only and do not authorize sessions.
pub fn routes() -> Router<AppState> {
    let bounded = DefaultBodyLimit::max(BOUNDED_JSON_BYTES);
    Router::new()
        .route("/api/heartbeat", post(heartbeat))
        .route("/api/sysinfo", post(sysinfo))
        .route("/api/audit/conn/active", get(active_connection_audit))
        .route(
            "/api/audit/conn",
            post(audit_connection).layer(bounded.clone()),
        )
        .route("/api/audit/file", post(audit_file).layer(bounded.clone()))
        .route(
            "/api/audit",
            post(audit_note).put(audit_note).layer(bounded.clone()),
        )
        .route("/api/record", post(record_upload).layer(bounded))
}

#[derive(Debug, Deserialize)]
struct HeartbeatRequest {
    id: String,
    uuid: String,
    ver: i64,
    conns: Option<Vec<i32>>,
    modified_at: i64,
}

#[derive(Debug, Deserialize)]
struct SysinfoRequest {
    id: String,
    uuid: String,
    version: String,
    #[serde(flatten)]
    fields: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AuditConnectionRequest {
    Open(AuditConnectionOpen),
    Authorized(AuditConnectionAuthorized),
    Close(AuditConnectionClose),
    Note(AuditConnectionNote),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditConnectionOpen {
    id: String,
    uuid: String,
    conn_id: i32,
    #[serde(rename = "session_id")]
    _session_id: u64,
    ip: String,
    action: String,
    conn_audit_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditConnectionAuthorized {
    id: String,
    uuid: String,
    conn_id: i32,
    #[serde(rename = "session_id")]
    _session_id: u64,
    peer: (String, String),
    #[serde(rename = "type")]
    connection_type: i32,
    primary_auth: Option<i64>,
    two_factor: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditConnectionClose {
    id: String,
    uuid: String,
    conn_id: i32,
    #[serde(rename = "session_id")]
    _session_id: u64,
    action: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditConnectionNote {
    id: String,
    #[serde(rename = "session_id")]
    _session_id: u64,
    note: String,
}

#[derive(Debug, Deserialize)]
struct RecordQuery {
    #[serde(rename = "type")]
    kind: Option<String>,
    file: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ActiveConnectionQuery {
    id: Option<String>,
    session_id: Option<String>,
    conn_type: Option<String>,
}

fn valid_text(value: &str, max_length: usize) -> bool {
    !value.is_empty() && value.len() <= max_length && !value.chars().any(char::is_control)
}

fn valid_heartbeat(request: &HeartbeatRequest) -> bool {
    valid_text(&request.id, MAX_ID_LENGTH)
        && valid_text(&request.uuid, MAX_UUID_LENGTH)
        && request.ver >= 0
        && request.modified_at >= 0
        && request.conns.as_ref().is_none_or(|connections| {
            connections.len() <= MAX_ARRAY_LENGTH && connections.iter().all(|id| *id >= 0)
        })
}

fn valid_json(value: &Value, depth: usize) -> bool {
    if depth > MAX_SYSINFO_DEPTH {
        return false;
    }
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => true,
        Value::String(value) => {
            value.len() <= MAX_TEXT_LENGTH && !value.chars().any(char::is_control)
        }
        Value::Array(values) => {
            values.len() <= MAX_ARRAY_LENGTH
                && values.iter().all(|value| valid_json(value, depth + 1))
        }
        Value::Object(values) => {
            values.len() <= MAX_SYSINFO_FIELDS
                && values.iter().all(|(key, value)| {
                    valid_text(key, MAX_VERSION_LENGTH) && valid_json(value, depth + 1)
                })
        }
    }
}

fn valid_sysinfo(request: &SysinfoRequest) -> bool {
    valid_text(&request.id, MAX_ID_LENGTH)
        && valid_text(&request.uuid, MAX_UUID_LENGTH)
        && valid_text(&request.version, MAX_VERSION_LENGTH)
        && request.fields.len() <= MAX_SYSINFO_FIELDS
        && request
            .fields
            .iter()
            .all(|(key, value)| valid_text(key, MAX_VERSION_LENGTH) && valid_json(value, 1))
}

fn valid_audit_common(id: &str, uuid: &str, conn_id: i32) -> bool {
    valid_text(id, MAX_ID_LENGTH) && valid_text(uuid, MAX_UUID_LENGTH) && conn_id >= 0
}

fn valid_audit_connection(request: &AuditConnectionRequest) -> bool {
    match request {
        AuditConnectionRequest::Open(request) => {
            valid_audit_common(&request.id, &request.uuid, request.conn_id)
                && valid_text(&request.ip, MAX_ID_LENGTH)
                && request.action == "new"
                && request.conn_audit_ref.as_deref().is_none_or(|value| {
                    value.len() <= MAX_TEXT_LENGTH && !value.chars().any(char::is_control)
                })
        }
        AuditConnectionRequest::Authorized(request) => {
            valid_audit_common(&request.id, &request.uuid, request.conn_id)
                && valid_text(&request.peer.0, MAX_ID_LENGTH)
                && request.peer.1.len() <= MAX_TEXT_LENGTH
                && !request.peer.1.chars().any(char::is_control)
                && (0..=4).contains(&request.connection_type)
                && request.primary_auth.is_none_or(|value| value >= 0)
                && request.two_factor.is_none_or(|value| value >= 0)
        }
        AuditConnectionRequest::Close(request) => {
            valid_audit_common(&request.id, &request.uuid, request.conn_id)
                && request.action == "close"
        }
        AuditConnectionRequest::Note(request) => {
            valid_text(&request.id, MAX_ID_LENGTH)
                && request.note.len() <= MAX_AUDIT_NOTE_LENGTH
                && !request.note.chars().any(char::is_control)
        }
    }
}

fn optional_text(value: Option<&Value>, max_length: usize) -> bool {
    value.is_none_or(|value| {
        value
            .as_str()
            .is_some_and(|text| valid_text(text, max_length))
    })
}

fn valid_audit_object(value: &Value) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    map.len() <= MAX_SYSINFO_FIELDS
        && valid_json(value, 1)
        && optional_text(map.get("id"), MAX_ID_LENGTH)
        && optional_text(map.get("guid"), MAX_UUID_LENGTH)
        && map.get("note").is_none_or(|note| {
            note.as_str().is_some_and(|text| {
                text.len() <= MAX_AUDIT_NOTE_LENGTH && !text.chars().any(char::is_control)
            })
        })
}

fn valid_audit_file(value: &Value) -> bool {
    valid_audit_object(value)
        && value
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| valid_text(id, MAX_ID_LENGTH))
        && optional_text(value.get("uuid"), MAX_UUID_LENGTH)
        && optional_text(value.get("peer_id"), MAX_ID_LENGTH)
        && optional_text(value.get("path"), MAX_TEXT_LENGTH)
}

fn valid_record_query(query: &RecordQuery) -> bool {
    query
        .kind
        .as_deref()
        .is_none_or(|kind| RECORD_TYPES.contains(&kind))
        && query
            .file
            .as_deref()
            .is_none_or(|file| valid_text(file, MAX_TEXT_LENGTH))
}

fn compatibility_error(status: StatusCode) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": "Invalid request" })))
}

async fn heartbeat(
    State(state): State<AppState>,
    Json(request): Json<HeartbeatRequest>,
) -> Result<Json<Value>, StatusCode> {
    if !valid_heartbeat(&request) {
        return Err(StatusCode::BAD_REQUEST);
    }
    persist_last_seen(&state.db, &request.id).await;
    Ok(Json(json!({})))
}

async fn sysinfo(
    State(state): State<AppState>,
    Json(request): Json<SysinfoRequest>,
) -> Result<&'static str, StatusCode> {
    if !valid_sysinfo(&request) {
        return Err(StatusCode::BAD_REQUEST);
    }
    persist_sysinfo(&state.db, &request).await;
    Ok("SYSINFO_UPDATED")
}

async fn audit_connection(
    State(state): State<AppState>,
    request: Result<Json<AuditConnectionRequest>, JsonRejection>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let Json(request) = request.map_err(|_| compatibility_error(StatusCode::BAD_REQUEST))?;
    if !valid_audit_connection(&request) {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    persist_connection_audit(&state.db, &request).await;
    Ok(StatusCode::OK)
}

async fn audit_note(
    State(state): State<AppState>,
    request: Result<Json<Value>, JsonRejection>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let Json(value) = request.map_err(|_| compatibility_error(StatusCode::BAD_REQUEST))?;
    if !valid_audit_object(&value) {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    persist_note(&state.db, &value).await;
    Ok(StatusCode::OK)
}

async fn audit_file(
    State(state): State<AppState>,
    request: Result<Json<Value>, JsonRejection>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let Json(value) = request.map_err(|_| compatibility_error(StatusCode::BAD_REQUEST))?;
    if !valid_audit_file(&value) {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    persist_file_transfer(&state.db, &value).await;
    Ok(StatusCode::OK)
}

async fn active_connection_audit(
    Query(query): Query<ActiveConnectionQuery>,
) -> Result<Json<String>, (StatusCode, Json<Value>)> {
    let rustdesk_id = query
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| valid_text(value, MAX_ID_LENGTH))
        .ok_or_else(|| compatibility_error(StatusCode::BAD_REQUEST))?;
    let session_id = query.session_id.as_deref().unwrap_or("").trim().to_string();
    if session_id.len() > MAX_UUID_LENGTH || session_id.chars().any(char::is_control) {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    if query
        .conn_type
        .as_deref()
        .is_some_and(|value| value.len() > MAX_ID_LENGTH || value.chars().any(char::is_control))
    {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    let name = format!("{rustdesk_id}:{session_id}");
    Ok(Json(
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name.as_bytes()).to_string(),
    ))
}

async fn record_upload(
    State(state): State<AppState>,
    Query(query): Query<RecordQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if !valid_record_query(&query) {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    persist_session_record(&state.db, &query).await;
    Ok(Json(json!({ "stored": false })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_validation_rejects_control_and_oversized_values() {
        assert!(!valid_text("bad\nvalue", MAX_ID_LENGTH));
        assert!(!valid_text(&"x".repeat(MAX_ID_LENGTH + 1), MAX_ID_LENGTH));
        assert!(!valid_json(
            &Value::String("x".repeat(MAX_TEXT_LENGTH + 1)),
            1
        ));
        assert!(!valid_record_query(&RecordQuery {
            kind: Some("video".to_string()),
            file: Some("clip.mp4".to_string()),
        }));
    }
}
