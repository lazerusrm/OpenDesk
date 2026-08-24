mod access_groups;
mod account;
mod address_books;
mod audit;
mod auth;
mod backup;
mod client_ab_password;
mod client_account;
mod client_address_book_mutations;
mod client_legacy_address_book;
mod client_login_options;
mod client_sync;
mod dashboard;
mod deployment;
mod device_export;
mod devices;
mod devices_deploy;
mod enrollment;
mod onboard;
mod render;
mod rustdesk_compat;
mod settings;
mod setup;
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
        .merge(dashboard::routes())
        .merge(account::routes())
        .merge(audit::routes())
        .merge(access_groups::routes())
        .merge(address_books::routes())
        .merge(backup::routes())
        .merge(client_account::routes())
        .merge(client_ab_password::routes())
        .merge(client_address_book_mutations::routes())
        .merge(client_legacy_address_book::routes())
        .merge(client_login_options::routes())
        .merge(client_sync::routes())
        .merge(devices::routes())
        .merge(settings::routes())
        .merge(setup::routes())
        .merge(sites::routes())
        .merge(tags::routes())
        .merge(deployment::routes())
        .merge(enrollment::routes())
        .merge(onboard::routes())
        .merge(devices_deploy::routes())
        .merge(rustdesk_compat::routes())
        .merge(status::routes())
        .merge(transport_introspection::routes())
        .merge(users::routes())
        .route("/health", get(|| async { "ok" }))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
