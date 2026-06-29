use askama::Template;
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Form, Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::domain::audit_event::AuditEventDraft;
use crate::domain::backup::{parse_backup_json, render_backup_json};
use crate::http::session::require_user;
use crate::http::views::BackupView;
use crate::repository::audit_events::insert_audit_event;
use crate::repository::backup::{export_backup_document, restore_backup_document};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/backup", get(backup_page).post(backup_restore_submit))
        .route("/backup/export.json", get(backup_export))
}

async fn backup_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let _user = require_user(&state, &jar).await?;
    Ok(render_backup_page(None, None).into_response())
}

async fn backup_export(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    let document = export_backup_document(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let json = render_backup_json(&document)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "backup_export".to_string(),
        object_type: "backup".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"opendesk-backup.json\"",
            ),
        ],
        json,
    )
        .into_response())
}

#[derive(Deserialize)]
struct BackupRestoreForm {
    backup_json: String,
    confirm: Option<String>,
}

async fn backup_restore_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<BackupRestoreForm>,
) -> Result<Response, Response> {
    let user = require_user(&state, &jar).await?;
    if form.confirm.as_deref() != Some("yes") {
        return Ok(render_backup_page(
            None,
            Some("Restore requires confirmation.".to_string()),
        )
        .into_response());
    }
    let document = match parse_backup_json(form.backup_json.trim()) {
        Ok(document) => document,
        Err(_) => {
            return Ok(render_backup_page(
                None,
                Some("Backup JSON is invalid.".to_string()),
            )
            .into_response());
        }
    };
    if let Err(error) = restore_backup_document(&state.db, &document).await {
        return Ok(render_backup_page(None, Some(error.to_string())).into_response());
    }
    let audit = AuditEventDraft {
        actor_user_uuid: Some(user.user_uuid),
        action: "backup_restore".to_string(),
        object_type: "backup".to_string(),
        object_uuid: None,
        outcome: "success".to_string(),
        source: "web".to_string(),
        detail: None,
    };
    let _ = insert_audit_event(&state.db, &audit).await;
    Ok(Redirect::to("/devices").into_response())
}

fn render_backup_page(message: Option<String>, error_message: Option<String>) -> Html<String> {
    let view = BackupView {
        title: "Backup".to_string(),
        show_nav: true,
        message,
        error_message,
    };
    Html(view.render().expect("render backup page"))
}