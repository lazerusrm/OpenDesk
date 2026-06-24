use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::site::{validate_site_draft, SiteDraft};
use crate::http::session::require_user;
use crate::http::views::{SiteRowView, SitesListView};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::sites::{create_site, list_sites};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sites", get(sites_list).post(site_create_submit))
}

#[derive(Deserialize)]
struct SiteForm {
    name: String,
}

async fn sites_list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    Ok(render_sites_page(&state, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response())
}

async fn site_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<SiteForm>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let draft = SiteDraft { name: form.name };
    if let Err(error) = validate_site_draft(&draft) {
        return Ok(render_sites_page(&state, Some(error.to_string()))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response());
    }
    let site = match create_site(&state.db, &draft).await {
        Ok(site) => site,
        Err(sqlx::Error::Database(db_error)) if db_error.is_unique_violation() => {
            return Ok(render_sites_page(
                &state,
                Some("site name already exists".to_string()),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response());
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    };
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "site_create".to_string(),
        object_type: "site".to_string(),
        object_uuid: Some(site.site_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(Redirect::to("/sites").into_response())
}

async fn render_sites_page(
    state: &AppState,
    error_message: Option<String>,
) -> Result<Html<String>, sqlx::Error> {
    let sites = list_sites(&state.db).await?;
    let rows = sites
        .into_iter()
        .map(|site| SiteRowView {
            site_uuid: site.site_uuid.to_string(),
            name: site.name,
        })
        .collect();
    let view = SitesListView {
        title: "Sites".to_string(),
        show_nav: true,
        sites: rows,
        error_message,
    };
    Ok(Html(view.render().expect("render sites list")))
}