//! Isolated official-client compatibility for `/api/devices/deploy` and
//! `/api/devices/cli`. Bearer values are OpenDesk enrollment tokens hashed
//! with `hash_enrollment_token_value`. This boundary does not claim session
//! enforcement and does not persist client public keys or address-book passwords.

#[path = "devices_deploy_body.rs"]
mod devices_deploy_body;

use devices_deploy_body::{parse_cli_body, parse_deploy_body};

use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use super::client_account::bearer_token;
use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::device::{normalize_device_draft, validate_device_draft, Device, DeviceDraft};
use crate::domain::enrollment_checkin::{
    apply_cli_alias_notes, hostname_lookup_key, select_existing_device_for_checkin,
    EnrollmentDeviceLookup,
};
use crate::domain::enrollment_token::{
    hash_enrollment_token_value, verify_enrollment_token_value, EnrollmentTokenRecord,
};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::devices::{
    create_device, find_device_by_hostname, find_device_by_rustdesk_id, touch_device_checkin,
    update_device,
};
use crate::repository::enrollment_tokens::{
    find_enrollment_token_by_hash, grant_issuer_device_visibility, record_endpoint_checkin,
};

const CLI_ERROR: &str = "Invalid input";
const ADAPTER_DEPLOY: &str = "devices/deploy";
const ADAPTER_CLI: &str = "devices/cli";

#[derive(Serialize)]
struct DeployResult {
    result: &'static str,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/devices/deploy", post(devices_deploy))
        .route("/api/devices/cli", post(devices_cli))
        .layer(DefaultBodyLimit::max(16 * 1024))
}

fn deploy_result(result: &'static str) -> Json<DeployResult> {
    Json(DeployResult { result })
}

enum CliOutcome {
    Done,
    Invalid,
}

impl IntoResponse for CliOutcome {
    fn into_response(self) -> Response {
        match self {
            CliOutcome::Done => StatusCode::OK.into_response(),
            CliOutcome::Invalid => (StatusCode::OK, CLI_ERROR).into_response(),
        }
    }
}

async fn devices_deploy(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Json<DeployResult> {
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            audit(
                &state,
                None,
                "failure",
                json!({"reason":"invalid_json","adapter":ADAPTER_DEPLOY}),
            )
            .await;
            return deploy_result("INVALID_INPUT");
        }
    };
    let Some(token) = bearer_token(&headers) else {
        audit(
            &state,
            None,
            "failure",
            json!({"reason":"missing_or_invalid_bearer","adapter":ADAPTER_DEPLOY}),
        )
        .await;
        return deploy_result("INVALID_INPUT");
    };
    let Some(record) = resolve_token(&state, token, ADAPTER_DEPLOY).await else {
        return deploy_result("INVALID_INPUT");
    };
    let Some((rustdesk_id, hostname)) = parse_deploy_body(&body) else {
        audit(
            &state,
            None,
            "failure",
            json!({
                "reason": "invalid_device_payload",
                "adapter": ADAPTER_DEPLOY,
                "enrollment_token": token,
            }),
        )
        .await;
        return deploy_result("INVALID_INPUT");
    };
    let alias = hostname.clone().unwrap_or_else(|| rustdesk_id.clone());
    let draft = DeviceDraft {
        rustdesk_id: Some(rustdesk_id),
        alias,
        hostname,
        site_uuid: record.site_uuid,
        ..Default::default()
    };
    match enroll(&state, &record, draft, token, ADAPTER_DEPLOY).await {
        Ok(_) => deploy_result("OK"),
        Err(()) => deploy_result("INVALID_INPUT"),
    }
}

async fn devices_cli(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> CliOutcome {
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => {
            audit(
                &state,
                None,
                "failure",
                json!({"reason":"invalid_json","adapter":ADAPTER_CLI}),
            )
            .await;
            return CliOutcome::Invalid;
        }
    };
    let Some(token) = bearer_token(&headers) else {
        audit(
            &state,
            None,
            "failure",
            json!({"reason":"missing_or_invalid_bearer","adapter":ADAPTER_CLI}),
        )
        .await;
        return CliOutcome::Invalid;
    };
    let Some(record) = resolve_token(&state, token, ADAPTER_CLI).await else {
        return CliOutcome::Invalid;
    };
    let Some((rustdesk_id, device_name, note)) = parse_cli_body(&body) else {
        audit(
            &state,
            None,
            "failure",
            json!({
                "reason": "invalid_device_payload",
                "adapter": ADAPTER_CLI,
                "enrollment_token": token,
            }),
        )
        .await;
        return CliOutcome::Invalid;
    };
    let alias = device_name.clone().unwrap_or_else(|| rustdesk_id.clone());
    let draft = DeviceDraft {
        rustdesk_id: Some(rustdesk_id),
        alias,
        notes: note.clone(),
        site_uuid: record.site_uuid,
        ..Default::default()
    };
    let (device, is_update) = match enroll(&state, &record, draft, token, ADAPTER_CLI).await {
        Ok(result) => result,
        Err(()) => return CliOutcome::Invalid,
    };
    if is_update && (device_name.is_some() || note.is_some()) {
        let overlay = apply_cli_alias_notes(&device, device_name.as_deref(), note.as_deref());
        if update_device(&state.db, device.device_uuid, &overlay)
            .await
            .is_err()
        {
            return CliOutcome::Invalid;
        }
    }
    CliOutcome::Done
}

async fn resolve_token(
    state: &AppState,
    token: &str,
    adapter: &str,
) -> Option<EnrollmentTokenRecord> {
    let token_hash = hash_enrollment_token_value(token);
    let record = match find_enrollment_token_by_hash(&state.db, &token_hash).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            audit(
                state,
                None,
                "failure",
                json!({
                    "reason": "unknown_token",
                    "adapter": adapter,
                    "enrollment_token": token,
                }),
            )
            .await;
            return None;
        }
        Err(_) => return None,
    };
    if verify_enrollment_token_value(&record, token, OffsetDateTime::now_utc()).is_err() {
        audit(
            state,
            None,
            "failure",
            json!({
                "reason": "token_invalid_or_revoked",
                "adapter": adapter,
                "enrollment_token": token,
                "enrollment_token_uuid": record.enrollment_token_uuid,
            }),
        )
        .await;
        return None;
    }
    Some(record)
}

async fn enroll(
    state: &AppState,
    record: &EnrollmentTokenRecord,
    draft: DeviceDraft,
    token: &str,
    adapter: &str,
) -> Result<(Device, bool), ()> {
    let draft = normalize_device_draft(draft);
    if validate_device_draft(&draft).is_err() {
        audit(
            state,
            None,
            "failure",
            json!({
                "reason": "invalid_device_payload",
                "adapter": adapter,
                "enrollment_token": token,
            }),
        )
        .await;
        return Err(());
    }
    let by_rustdesk_id = if let Some(rustdesk_id) = draft.rustdesk_id.as_deref() {
        find_device_by_rustdesk_id(&state.db, rustdesk_id)
            .await
            .map_err(|_| ())?
    } else {
        None
    };
    let by_hostname = if let Some(hostname) = hostname_lookup_key(draft.hostname.as_deref()) {
        find_device_by_hostname(&state.db, &hostname)
            .await
            .map_err(|_| ())?
    } else {
        None
    };
    // Existing rustdesk_id is treated as check-in (OK). OpenDesk does not persist
    // official-client uuid, so ID_TAKEN is not returned for a colliding id.
    let existing_uuid = select_existing_device_for_checkin(&EnrollmentDeviceLookup {
        by_rustdesk_id,
        by_hostname,
    });
    let is_update = existing_uuid.is_some();
    let device = if let Some(device_uuid) = existing_uuid {
        touch_device_checkin(&state.db, device_uuid, &draft)
            .await
            .map_err(|_| ())?
    } else {
        let created = create_device(&state.db, &draft).await.map_err(|_| ())?;
        touch_device_checkin(&state.db, created.device_uuid, &draft)
            .await
            .map_err(|_| ())?
    };
    record_endpoint_checkin(
        &state.db,
        device.device_uuid,
        record.enrollment_token_uuid,
        draft.rustdesk_id.as_deref(),
        draft.hostname.as_deref(),
        None,
        None,
        None,
        None,
    )
    .await
    .map_err(|_| ())?;
    grant_issuer_device_visibility(&state.db, record, device.device_uuid)
        .await
        .map_err(|_| ())?;
    let action = if is_update {
        "endpoint_checkin_update"
    } else {
        "endpoint_checkin_create"
    };
    audit(
        state,
        Some(device.device_uuid),
        "success",
        json!({
            "action": action,
            "adapter": adapter,
            "enrollment_token_uuid": record.enrollment_token_uuid,
            "hostname": draft.hostname,
        }),
    )
    .await;
    Ok((device, is_update))
}

async fn audit(state: &AppState, device_uuid: Option<Uuid>, outcome: &str, detail: Value) {
    let draft = AuditEventDraft {
        actor_user_uuid: None,
        action: "endpoint_checkin".to_string(),
        object_type: "device".to_string(),
        object_uuid: device_uuid,
        outcome: outcome.to_string(),
        source: "api".to_string(),
        detail: Some(detail),
    };
    let _ = insert_audit_event(&state.db, &draft).await;
}
