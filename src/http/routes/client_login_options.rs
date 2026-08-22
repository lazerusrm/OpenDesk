use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

use crate::app_state::AppState;

const EMPTY_OIDC_LOGIN_OPTION: &str = "common-oidc/[]";

/// Isolated official-client discovery for login methods.
/// Flutter GET `{api}/api/login-options` JSON-decodes an array; empty-OIDC
/// RustDesk Pro returns `["common-oidc/[]"]` so queryOidcLoginOptions parses to [].
/// These handlers do not enable OIDC authentication.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/login-options", get(login_options))
        .route("/api/oidc/info", get(oidc_info))
        .route("/api/oidc/login-options", get(oidc_login_options))
}

async fn login_options() -> Json<Vec<&'static str>> {
    Json(vec![EMPTY_OIDC_LOGIN_OPTION])
}

async fn oidc_info() -> Json<Value> {
    Json(json!({}))
}

async fn oidc_login_options() -> Json<Vec<Value>> {
    Json(Vec::new())
}
