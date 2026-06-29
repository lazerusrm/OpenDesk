mod auth;
mod backup;
mod deployment;
mod device_export;
mod devices;
mod enrollment;
mod render;
mod settings;
mod status;
mod sites;
mod tags;

use axum::{routing::get, Router};
use tower_http::services::ServeDir;

use crate::app_state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(backup::routes())
        .merge(devices::routes())
        .merge(settings::routes())
        .merge(sites::routes())
        .merge(tags::routes())
        .merge(deployment::routes())
        .merge(enrollment::routes())
        .merge(status::routes())
        .route("/health", get(|| async { "ok" }))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}