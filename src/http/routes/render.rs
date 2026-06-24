use askama::Template;
use axum::response::Html;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::device::DeviceDraft;
use crate::domain::enrollment_token::EnrollmentTokenRecord;
use crate::domain::server_config::ServerConfig;
use crate::http::views::{
    DeviceFormView, EnrollmentTokenRowView, EnrollmentTokensView, LoginView, ServerConfigView,
    SiteOptionView, TagOptionView,
};
use crate::repository::enrollment_tokens::list_enrollment_tokens;
use crate::repository::sites::list_sites;
use crate::repository::tags::list_tags;

pub fn render_login(error_message: Option<String>) -> Html<String> {
    let view = LoginView {
        title: "Login".to_string(),
        show_nav: false,
        error_message,
    };
    Html(view.render().expect("render login"))
}

pub async fn render_device_form(
    state: &AppState,
    heading: &str,
    form_action: &str,
    device_uuid: Uuid,
    draft: DeviceDraft,
    selected_tag_uuids: &[Uuid],
    error_message: Option<String>,
    show_archive_actions: bool,
    show_unarchive_actions: bool,
) -> Result<Html<String>, sqlx::Error> {
    let sites = list_sites(&state.db).await?;
    let site_options = sites
        .into_iter()
        .map(|site| SiteOptionView {
            site_uuid: site.site_uuid.to_string(),
            name: site.name,
            selected: Some(site.site_uuid) == draft.site_uuid,
        })
        .collect();
    let tags = list_tags(&state.db).await?;
    let tag_options = tags
        .into_iter()
        .map(|tag| TagOptionView {
            tag_uuid: tag.tag_uuid.to_string(),
            name: tag.name,
            selected: selected_tag_uuids.contains(&tag.tag_uuid),
        })
        .collect();
    let view = DeviceFormView {
        title: heading.to_string(),
        show_nav: true,
        heading: heading.to_string(),
        form_action: form_action.to_string(),
        device_uuid: device_uuid.to_string(),
        alias: draft.alias,
        rustdesk_id: draft.rustdesk_id.unwrap_or_default(),
        hostname: draft.hostname.unwrap_or_default(),
        owner: draft.owner.unwrap_or_default(),
        notes: draft.notes.unwrap_or_default(),
        site_options,
        tag_options,
        error_message,
        show_archive_actions,
        show_unarchive_actions,
    };
    Ok(Html(view.render().expect("render device form")))
}

pub fn render_server_config(
    config: &ServerConfig,
    message: Option<String>,
    error_message: Option<String>,
) -> Html<String> {
    let view = ServerConfigView {
        title: "Server Config".to_string(),
        show_nav: true,
        id_server: config.id_server.clone(),
        relay_server: config.relay_server.clone(),
        api_server: config.api_server.clone(),
        public_key: config.public_key.clone(),
        message,
        error_message,
    };
    Html(view.render().expect("render server config"))
}

pub async fn render_enrollment_tokens(
    state: &AppState,
    created_token_value: Option<String>,
) -> Result<Html<String>, sqlx::Error> {
    let tokens = list_enrollment_tokens(&state.db).await?;
    let rows = tokens
        .into_iter()
        .map(|token| {
            let status = enrollment_token_status(&token);
            EnrollmentTokenRowView {
                enrollment_token_uuid: token.enrollment_token_uuid.to_string(),
                label: token.label,
                status,
                can_revoke: token.revoked_at.is_none(),
            }
        })
        .collect();
    let view = EnrollmentTokensView {
        title: "Enrollment Tokens".to_string(),
        show_nav: true,
        tokens: rows,
        created_token_value,
    };
    Ok(Html(view.render().expect("render enrollment tokens")))
}

pub fn enrollment_token_status(token: &EnrollmentTokenRecord) -> String {
    if token.revoked_at.is_some() {
        "revoked".to_string()
    } else if token
        .expires_at
        .is_some_and(|expires| OffsetDateTime::now_utc() >= expires)
    {
        "expired".to_string()
    } else {
        "active".to_string()
    }
}