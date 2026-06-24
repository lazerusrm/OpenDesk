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
use crate::domain::tag::{validate_tag_draft, TagDraft};
use crate::http::session::require_user;
use crate::http::views::{TagRowView, TagsListView};
use crate::repository::audit_events::insert_audit_event;
use crate::repository::tags::{create_tag, list_tags};

pub fn routes() -> Router<AppState> {
    Router::new().route("/tags", get(tags_list).post(tag_create_submit))
}

#[derive(Deserialize)]
struct TagForm {
    name: String,
}

async fn tags_list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    Ok(render_tags_page(&state, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        .into_response())
}

async fn tag_create_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<TagForm>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let draft = TagDraft { name: form.name };
    if let Err(error) = validate_tag_draft(&draft) {
        return Ok(render_tags_page(&state, Some(error.to_string()))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response());
    }
    let tag = match create_tag(&state.db, &draft).await {
        Ok(tag) => tag,
        Err(sqlx::Error::Database(db_error)) if db_error.is_unique_violation() => {
            return Ok(render_tags_page(
                &state,
                Some("tag name already exists".to_string()),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
            .into_response());
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
    };
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "tag_create".to_string(),
        object_type: "tag".to_string(),
        object_uuid: Some(tag.tag_uuid),
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(Redirect::to("/tags").into_response())
}

async fn render_tags_page(
    state: &AppState,
    error_message: Option<String>,
) -> Result<Html<String>, sqlx::Error> {
    let tags = list_tags(&state.db).await?;
    let rows = tags
        .into_iter()
        .map(|tag| TagRowView {
            tag_uuid: tag.tag_uuid.to_string(),
            name: tag.name,
        })
        .collect();
    let view = TagsListView {
        title: "Tags".to_string(),
        show_nav: true,
        tags: rows,
        error_message,
    };
    Ok(Html(view.render().expect("render tags list")))
}