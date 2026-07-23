mod access_groups;
mod address_books;
mod audit;
mod auth;
mod backup;
mod client_account;
mod client_address_book_mutations;
mod client_legacy_address_book;
mod client_sync;
mod deployment;
mod device_export;
mod devices;
mod enrollment;
mod render;
mod rustdesk_compat;
mod settings;
mod sites;
mod status;
mod tags;
mod transport_introspection;
mod users;

use axum::{routing::get, Router};
use tower_http::services::ServeDir;

use crate::app_state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(auth::routes())
        .merge(audit::routes())
        .merge(access_groups::routes())
        .merge(address_books::routes())
        .merge(backup::routes())
        .merge(client_account::routes())
        .merge(client_address_book_mutations::routes())
        .merge(client_legacy_address_book::routes())
        .merge(client_sync::routes())
        .merge(devices::routes())
        .merge(settings::routes())
        .merge(sites::routes())
        .merge(tags::routes())
        .merge(deployment::routes())
        .merge(enrollment::routes())
        .merge(rustdesk_compat::routes())
        .merge(status::routes())
        .merge(transport_introspection::routes())
        .merge(users::routes())
        .route("/health", get(|| async { "ok" }))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
