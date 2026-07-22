use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Form, Json, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::access_policy::Action;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::device::{normalize_device_draft, validate_device_draft, DeviceDraft};
use crate::domain::enrollment_checkin::{
    hostname_lookup_key, select_existing_device_for_checkin, EnrollmentDeviceLookup,
};
use crate::domain::enrollment_token::{
    hash_enrollment_token_value, validate_enrollment_token_label, verify_enrollment_token_value,
};
use crate::http::routes::render::render_enrollment_tokens;
use crate::http::session::{require_action, require_csrf};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::devices::{
    create_device, find_device_by_hostname, find_device_by_rustdesk_id, touch_device_checkin,
};
use crate::repository::enrollment_tokens::{
    create_enrollment_token, find_enrollment_token_by_hash, record_endpoint_checkin,
    revoke_enrollment_token,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/enrollment-tokens",
            axum::routing::get(enrollment_tokens_page).post(enrollment_token_create),
        )
        .route(
            "/enrollment-tokens/{enrollment_token_uuid}/revoke",
            post(enrollment_token_revoke),
        )
        .route("/api/enrollments/check-in", post(enrollment_checkin))
}

async fn enrollment_tokens_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenList).await?;
    Ok(
        render_enrollment_tokens(&state, None, &user.csrf_token, user.parsed_role())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response(),
    )
}

#[derive(Deserialize)]
struct EnrollmentTokenForm {
    csrf_token: String,
    label: String,
}

async fn enrollment_token_create(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<EnrollmentTokenForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenCreate).await?;
    require_csrf(&user, &form.csrf_token)?;
    if let Err(_error) = validate_enrollment_token_label(&form.label) {
        return Ok(
            render_enrollment_tokens(&state, None, &user.csrf_token, user.parsed_role())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
                .into_response(),
        );
    }
    let created = create_enrollment_token(
        &state.db,
        form.label.trim(),
        None,
        None,
        Some(user.user_uuid),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "enrollment_token_create".to_string(),
        object_type: "enrollment_token".to_string(),
        object_uuid: Some(created.record.enrollment_token_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(render_enrollment_tokens(
        &state,
        Some(created.token_value),
        &user.csrf_token,
        user.parsed_role(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    .into_response())
}

#[derive(Deserialize)]
struct EnrollmentTokenRevokeForm {
    csrf_token: String,
}

async fn enrollment_token_revoke(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(enrollment_token_uuid): Path<Uuid>,
    Form(form): Form<EnrollmentTokenRevokeForm>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::EnrollmentTokenRevoke).await?;
    require_csrf(&user, &form.csrf_token)?;
    revoke_enrollment_token(&state.db, enrollment_token_uuid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "enrollment_token_revoke".to_string(),
        object_type: "enrollment_token".to_string(),
        object_uuid: Some(enrollment_token_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(
        render_enrollment_tokens(&state, None, &user.csrf_token, user.parsed_role())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response(),
    )
}

#[derive(Deserialize)]
struct EnrollmentCheckinRequest {
    enrollment_token: String,
    rustdesk_id: Option<String>,
    hostname: Option<String>,
    os_family: Option<String>,
    os_version: Option<String>,
    architecture: Option<String>,
    rustdesk_version: Option<String>,
}

async fn enrollment_checkin(
    State(state): State<AppState>,
    Json(body): Json<EnrollmentCheckinRequest>,
) -> Result<StatusCode, StatusCode> {
    let token_hash = hash_enrollment_token_value(&body.enrollment_token);
    let record = match find_enrollment_token_by_hash(&state.db, &token_hash).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            write_checkin_audit(
                &state,
                None,
                "failure",
                Some(serde_json::json!({
                    "reason": "unknown_token",
                    "enrollment_token": body.enrollment_token,
                    "hostname": body.hostname,
                })),
            )
            .await;
            return Err(StatusCode::UNAUTHORIZED);
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    if verify_enrollment_token_value(&record, &body.enrollment_token, OffsetDateTime::now_utc())
        .is_err()
    {
        write_checkin_audit(
            &state,
            None,
            "failure",
            Some(serde_json::json!({
                "reason": "token_invalid_or_revoked",
                "enrollment_token": body.enrollment_token,
                "enrollment_token_uuid": record.enrollment_token_uuid,
            })),
        )
        .await;
        return Err(StatusCode::UNAUTHORIZED);
    }
    let draft = normalize_device_draft(DeviceDraft {
        rustdesk_id: body.rustdesk_id.clone(),
        alias: body
            .hostname
            .clone()
            .or_else(|| body.rustdesk_id.clone())
            .unwrap_or_else(|| "enrolled-device".to_string()),
        hostname: body.hostname.clone(),
        os_family: body.os_family.clone(),
        os_version: body.os_version.clone(),
        architecture: body.architecture.clone(),
        rustdesk_version: body.rustdesk_version.clone(),
        site_uuid: record.site_uuid,
        owner: None,
        notes: None,
    });
    if validate_device_draft(&draft).is_err() {
        write_checkin_audit(
            &state,
            None,
            "failure",
            Some(serde_json::json!({
                "reason": "invalid_device_payload",
                "enrollment_token": body.enrollment_token,
            })),
        )
        .await;
        return Err(StatusCode::BAD_REQUEST);
    }
    let by_rustdesk_id = if let Some(rustdesk_id) = draft.rustdesk_id.as_deref() {
        find_device_by_rustdesk_id(&state.db, rustdesk_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        None
    };
    let by_hostname = if let Some(hostname) = hostname_lookup_key(draft.hostname.as_deref()) {
        find_device_by_hostname(&state.db, &hostname)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        None
    };
    let existing_uuid = select_existing_device_for_checkin(&EnrollmentDeviceLookup {
        by_rustdesk_id,
        by_hostname,
    });
    let is_update = existing_uuid.is_some();
    let device = if let Some(device_uuid) = existing_uuid {
        touch_device_checkin(&state.db, device_uuid, &draft)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        let created = create_device(&state.db, &draft)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        touch_device_checkin(&state.db, created.device_uuid, &draft)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    record_endpoint_checkin(
        &state.db,
        device.device_uuid,
        record.enrollment_token_uuid,
        body.rustdesk_id.as_deref(),
        body.hostname.as_deref(),
        body.os_family.as_deref(),
        body.os_version.as_deref(),
        body.architecture.as_deref(),
        body.rustdesk_version.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let action = if is_update {
        "endpoint_checkin_update"
    } else {
        "endpoint_checkin_create"
    };
    write_checkin_audit(
        &state,
        Some(device.device_uuid),
        "success",
        Some(serde_json::json!({
            "action": action,
            "enrollment_token_uuid": record.enrollment_token_uuid,
            "hostname": body.hostname,
            "os_family": body.os_family,
        })),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn write_checkin_audit(
    state: &AppState,
    device_uuid: Option<Uuid>,
    outcome: &str,
    detail: Option<serde_json::Value>,
) {
    let audit = AuditEventDraft {
        actor_user_uuid: None,
        action: "endpoint_checkin".to_string(),
        object_type: "device".to_string(),
        object_uuid: device_uuid,
        outcome: outcome.to_string(),
        source: "api".to_string(),
        detail,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
}
