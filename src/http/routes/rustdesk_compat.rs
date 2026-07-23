use std::collections::HashMap;

use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app_state::AppState;

const MAX_ID_LENGTH: usize = 64;
const MAX_UUID_LENGTH: usize = 128;
const MAX_VERSION_LENGTH: usize = 128;
const MAX_SYSINFO_FIELDS: usize = 64;
const MAX_SYSINFO_DEPTH: usize = 4;
const MAX_TEXT_LENGTH: usize = 1024;
const MAX_ARRAY_LENGTH: usize = 128;
const MAX_AUDIT_NOTE_LENGTH: usize = 1024;

/// RustDesk-shaped compatibility boundary for official-client liveness posts.
/// It acknowledges bounded payloads but does not authenticate dashboard access,
/// enroll devices, store client metadata, or return connection-control policy.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/heartbeat", post(heartbeat))
        .route("/api/sysinfo", post(sysinfo))
        .route(
            "/api/audit/conn",
            post(audit_connection).layer(DefaultBodyLimit::max(16 * 1024)),
        )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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

async fn audit_connection(
    request: Result<Json<AuditConnectionRequest>, JsonRejection>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let Json(request) = request.map_err(|_| compatibility_error(StatusCode::BAD_REQUEST))?;
    let valid = match request {
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
    };
    if !valid {
        return Err(compatibility_error(StatusCode::BAD_REQUEST));
    }
    Ok(StatusCode::OK)
}

fn valid_audit_common(id: &str, uuid: &str, conn_id: i32) -> bool {
    valid_text(id, MAX_ID_LENGTH) && valid_text(uuid, MAX_UUID_LENGTH) && conn_id >= 0
}

fn compatibility_error(status: StatusCode) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": "Invalid request" })))
}

async fn heartbeat(Json(request): Json<HeartbeatRequest>) -> Result<Json<Value>, StatusCode> {
    if !valid_heartbeat(&request) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(json!({})))
}

async fn sysinfo(Json(request): Json<SysinfoRequest>) -> Result<&'static str, StatusCode> {
    if !valid_sysinfo(&request) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok("SYSINFO_UPDATED")
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
    }
}
