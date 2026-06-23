mod auth;
mod deployment;
mod devices;
mod enrollment;
mod render;
mod settings;

use axum::{routing::get, Router};
use tower_http::services::ServeDir;

use crate::app_state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(devices::routes())
        .merge(settings::routes())
        .merge(deployment::routes())
        .merge(enrollment::routes())
        .route("/health", get(|| async { "ok" }))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}