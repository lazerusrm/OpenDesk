use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::client_access_token::validate_client_identity;
use crate::login_throttle::request_ip;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::{
    client_access_tokens::{
        authenticate_client_access_token, authenticate_client_bearer_token,
        issue_client_access_token, revoke_client_access_token, AuthenticatedClientToken,
    },
    users::{authenticate_user_password, find_user_by_uuid, UserRow},
};

const ACCOUNT_REQUEST_TYPE: &str = "account";
const ACCESS_TOKEN_RESPONSE_TYPE: &str = "access_token";
const INVALID_REQUEST: &str = "Invalid request";
const INVALID_CREDENTIALS: &str = "Invalid username or password";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/login", post(login))
        .route(
            "/api/currentUser",
            get(current_user_from_bearer).post(current_user),
        )
        .route("/api/logout", post(logout))
        .layer(DefaultBodyLimit::max(16 * 1024))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    username: Option<String>,
    password: Option<String>,
    id: Option<String>,
    uuid: Option<String>,
    #[serde(rename = "autoLogin")]
    _auto_login: Option<bool>,
    #[serde(rename = "type")]
    request_type: Option<String>,
    #[serde(rename = "verificationCode")]
    verification_code: Option<String>,
    #[serde(rename = "tfaCode")]
    tfa_code: Option<String>,
    secret: Option<String>,
    #[serde(rename = "deviceInfo")]
    device_info: Option<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientIdentityRequest {
    id: String,
    uuid: String,
}

#[derive(Serialize)]
struct ClientUserResponse {
    name: String,
    display_name: String,
    avatar: String,
    email: String,
    note: String,
    status: i32,
    info: Value,
    is_admin: bool,
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let Json(request) =
        request.map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))?;
    let (username, password, rustdesk_id, client_uuid) = accepted_login(&request)?;
    let ip = request_ip(&headers);
    let now = OffsetDateTime::now_utc();
    if state
        .login_throttle
        .lock()
        .map(|mut throttle| throttle.is_blocked(username, &ip, now))
        .unwrap_or(false)
    {
        return Err(json_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many sign-in attempts",
        ));
    }
    let user = match authenticate_user_password(&state.db, username, password)
        .await
        .map_err(internal_error)?
    {
        Some(user) => user,
        None => {
            if let Ok(mut throttle) = state.login_throttle.lock() {
                throttle.record_failure(username, &ip, now);
            }
            let audit = AuditEventDraft {
                actor_user_uuid: None,
                action: "login".to_string(),
                object_type: "session".to_string(),
                object_uuid: None,
                outcome: "failure".to_string(),
                source: "client".to_string(),
                detail: None,
            };
            let _ = insert_audit_event(&state.db, &audit).await;
            return Err(json_error(StatusCode::UNAUTHORIZED, INVALID_CREDENTIALS));
        }
    };
    if let Ok(mut throttle) = state.login_throttle.lock() {
        throttle.record_success(username, &ip);
    }
    let access_token = issue_client_access_token(
        &state.db,
        &state.client_token_hmac_key,
        user.user_uuid,
        rustdesk_id,
        client_uuid,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(json!({
        "access_token": access_token,
        "type": ACCESS_TOKEN_RESPONSE_TYPE,
        "user": client_user(&user),
    })))
}

async fn current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<ClientIdentityRequest>, JsonRejection>,
) -> Result<Json<ClientUserResponse>, (StatusCode, Json<Value>)> {
    let Json(request) =
        request.map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))?;
    validate_client_identity(&request.id, &request.uuid)
        .map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))?;
    let token = authenticate_request(&state, &headers, &request).await?;
    user_response(&state, token.user_uuid).await
}

async fn current_user_from_bearer(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ClientUserResponse>, (StatusCode, Json<Value>)> {
    let token = authenticate_bearer(&state, &headers).await?;
    user_response(&state, token.user_uuid).await
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<ClientIdentityRequest>, JsonRejection>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let Json(request) =
        request.map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))?;
    validate_client_identity(&request.id, &request.uuid)
        .map_err(|_| json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST))?;
    let token = authenticate_request(&state, &headers, &request).await?;
    revoke_client_access_token(
        &state.db,
        token.client_access_token_uuid,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(internal_error)?;
    Ok(Json(json!({})))
}

fn accepted_login(
    request: &LoginRequest,
) -> Result<(&str, &str, &str, &str), (StatusCode, Json<Value>)> {
    let accepted = request.request_type.as_deref() == Some(ACCOUNT_REQUEST_TYPE)
        && request.verification_code.is_none()
        && request.tfa_code.is_none()
        && request.secret.is_none()
        && request.device_info.as_ref().is_some_and(Value::is_object);
    let values = request
        .username
        .as_deref()
        .zip(request.password.as_deref())
        .zip(request.id.as_deref())
        .zip(request.uuid.as_deref());
    let Some((((username, password), rustdesk_id), client_uuid)) = values else {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    };
    if !accepted
        || username.trim().is_empty()
        || username.len() > 255
        || password.is_empty()
        || password.len() > 1024
        || validate_client_identity(rustdesk_id, client_uuid).is_err()
    {
        return Err(json_error(StatusCode::BAD_REQUEST, INVALID_REQUEST));
    }
    Ok((username.trim(), password, rustdesk_id, client_uuid))
}

async fn authenticate_request(
    state: &AppState,
    headers: &HeaderMap,
    request: &ClientIdentityRequest,
) -> Result<AuthenticatedClientToken, (StatusCode, Json<Value>)> {
    let value = bearer_token(headers).ok_or_else(unauthorized)?;
    authenticate_client_access_token(
        &state.db,
        &state.client_token_hmac_key,
        value,
        &request.id,
        &request.uuid,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(internal_error)?
    .ok_or_else(unauthorized)
}

async fn authenticate_bearer(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedClientToken, (StatusCode, Json<Value>)> {
    let value = bearer_token(headers).ok_or_else(unauthorized)?;
    authenticate_client_bearer_token(
        &state.db,
        &state.client_token_hmac_key,
        value,
        OffsetDateTime::now_utc(),
    )
    .await
    .map_err(internal_error)?
    .ok_or_else(unauthorized)
}

async fn user_response(
    state: &AppState,
    user_uuid: Uuid,
) -> Result<Json<ClientUserResponse>, (StatusCode, Json<Value>)> {
    let user = find_user_by_uuid(&state.db, user_uuid)
        .await
        .map_err(internal_error)?
        .ok_or_else(unauthorized)?;
    Ok(Json(client_user(&user)))
}

pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    (!token.is_empty() && !token.contains(char::is_whitespace)).then_some(token)
}

fn client_user(user: &UserRow) -> ClientUserResponse {
    ClientUserResponse {
        name: user.username.clone(),
        display_name: user.username.clone(),
        avatar: String::new(),
        email: String::new(),
        note: String::new(),
        status: 1,
        info: json!({}),
        is_admin: user.role == "admin",
    }
}

fn unauthorized() -> (StatusCode, Json<Value>) {
    json_error(StatusCode::UNAUTHORIZED, "Unauthorized")
}

fn internal_error(_: impl std::fmt::Debug) -> (StatusCode, Json<Value>) {
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}

pub(crate) fn json_error(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": message })))
}
