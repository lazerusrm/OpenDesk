use std::collections::HashMap;

use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::domain::audit_event::AuditEventDraft;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::devices::{
    find_device_by_rustdesk_id, touch_device_last_seen, touch_device_sysinfo,
};
use crate::time_format::format_timestamp;

use super::{AuditConnectionRequest, RecordQuery, SysinfoRequest, MAX_TEXT_LENGTH};

const SOURCE: &str = "official_client";
const OUTCOME: &str = "accepted";

pub(super) async fn persist_last_seen(pool: &SqlitePool, rustdesk_id: &str) {
    let now = format_timestamp(OffsetDateTime::now_utc());
    let _ = touch_device_last_seen(pool, rustdesk_id, &now).await;
}

pub(super) async fn persist_sysinfo(pool: &SqlitePool, request: &SysinfoRequest) {
    let now = format_timestamp(OffsetDateTime::now_utc());
    let hostname = string_field(&request.fields, "hostname");
    let os_family = string_field(&request.fields, "os");
    let _ = touch_device_sysinfo(
        pool,
        &request.id,
        hostname,
        os_family,
        Some(request.version.as_str()),
        &now,
    )
    .await;
}

pub(super) async fn persist_connection_audit(pool: &SqlitePool, request: &AuditConnectionRequest) {
    let (action, rustdesk_id, detail) = match request {
        AuditConnectionRequest::Open(request) => (
            "client_connection_open",
            request.id.as_str(),
            json!({
                "rustdesk_id": request.id,
                "uuid": request.uuid,
                "conn_id": request.conn_id,
                "ip": request.ip,
                "action": request.action,
                "conn_audit_ref": request.conn_audit_ref,
            }),
        ),
        AuditConnectionRequest::Authorized(request) => (
            "client_connection_authorized",
            request.id.as_str(),
            json!({
                "rustdesk_id": request.id,
                "uuid": request.uuid,
                "conn_id": request.conn_id,
                "peer_id": request.peer.0,
                "peer_name": request.peer.1,
                "type": request.connection_type,
            }),
        ),
        AuditConnectionRequest::Close(request) => (
            "client_connection_close",
            request.id.as_str(),
            json!({
                "rustdesk_id": request.id,
                "uuid": request.uuid,
                "conn_id": request.conn_id,
                "action": request.action,
            }),
        ),
        AuditConnectionRequest::Note(request) => (
            "client_connection_note",
            request.id.as_str(),
            json!({
                "rustdesk_id": request.id,
                "note": request.note,
            }),
        ),
    };
    persist_event(pool, action, Some(rustdesk_id), detail).await;
}

pub(super) async fn persist_note(pool: &SqlitePool, value: &Value) {
    persist_event(
        pool,
        "client_audit_note",
        value.get("id").and_then(Value::as_str),
        value.clone(),
    )
    .await;
}

pub(super) async fn persist_file_transfer(pool: &SqlitePool, value: &Value) {
    persist_event(
        pool,
        "client_file_transfer",
        value.get("id").and_then(Value::as_str),
        decode_embedded_info(value.clone()),
    )
    .await;
}

pub(super) async fn persist_session_record(pool: &SqlitePool, query: &RecordQuery) {
    if query.kind.as_deref() == Some("part") {
        return;
    }
    persist_event(
        pool,
        "client_session_record",
        None,
        json!({
            "type": query.kind,
            "file": query.file,
            "stored": false,
        }),
    )
    .await;
}

async fn persist_event(pool: &SqlitePool, action: &str, rustdesk_id: Option<&str>, detail: Value) {
    let (object_type, object_uuid) = match rustdesk_id.filter(|id| !id.is_empty()) {
        Some(rustdesk_id) => match find_device_by_rustdesk_id(pool, rustdesk_id).await {
            Ok(Some(device)) => ("device".to_string(), Some(device.device_uuid)),
            _ => ("client".to_string(), None),
        },
        None => ("client".to_string(), None),
    };
    let draft = AuditEventDraft {
        actor_user_uuid: None,
        action: action.to_string(),
        object_type,
        object_uuid,
        outcome: OUTCOME.to_string(),
        source: SOURCE.to_string(),
        detail: Some(detail),
    };
    let _ = insert_audit_event(pool, &draft).await;
}

fn string_field<'a>(fields: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= MAX_TEXT_LENGTH)
}

fn decode_embedded_info(mut detail: Value) -> Value {
    let parsed = match detail.get("info") {
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw).ok(),
        _ => None,
    };
    if let (Some(parsed), Value::Object(map)) = (parsed, &mut detail) {
        map.insert("info".to_string(), parsed);
    }
    if let Value::Object(map) = &mut detail {
        drop_blob_keys(map);
    }
    detail
}

fn drop_blob_keys(map: &mut Map<String, Value>) {
    for key in ["content", "data", "bytes", "file_content", "recording"] {
        map.remove(key);
    }
}
