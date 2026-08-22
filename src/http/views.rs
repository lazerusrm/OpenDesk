pub use super::deployment_views::{DeploymentDownloadView, DeploymentView};
use crate::domain::access_policy::Action;
use crate::domain::role::Role;
use askama::Template;
#[derive(Clone, Copy)]
pub struct NavPermissions {
    pub dashboard: bool,
    pub devices: bool,
    pub sites: bool,
    pub tags: bool,
    pub server_config: bool,
    pub deployment: bool,
    pub status: bool,
    pub backup: bool,
    pub enrollment: bool,
    pub audit: bool,
    pub users: bool,
    pub access_groups: bool,
    pub address_books: bool,
}
impl NavPermissions {
    pub const NONE: Self = Self {
        dashboard: false,
        devices: false,
        sites: false,
        tags: false,
        server_config: false,
        deployment: false,
        status: false,
        backup: false,
        enrollment: false,
        audit: false,
        users: false,
        access_groups: false,
        address_books: false,
    };
}
pub fn nav_permissions_for_role(role: Role) -> NavPermissions {
    NavPermissions {
        dashboard: Action::StatusView.allowed_for(role) || Action::DeviceList.allowed_for(role),
        devices: Action::DeviceList.allowed_for(role),
        sites: Action::SiteList.allowed_for(role),
        tags: Action::TagList.allowed_for(role),
        server_config: Action::ServerConfigView.allowed_for(role),
        deployment: Action::DeploymentView.allowed_for(role),
        status: Action::StatusView.allowed_for(role),
        backup: Action::BackupView.allowed_for(role),
        enrollment: Action::EnrollmentTokenList.allowed_for(role),
        audit: Action::AuditView.allowed_for(role),
        users: Action::UserList.allowed_for(role),
        access_groups: Action::AccessGroupList.allowed_for(role),
        address_books: Action::AddressBookList.allowed_for(role),
    }
}
#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub error_message: Option<String>,
    pub notice_message: Option<String>,
}
#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub username: String,
    pub role_display: String,
    pub error_message: Option<String>,
    pub can_manage_onboard_totp: bool,
    pub onboard_totp_enrolled: bool,
    pub onboard_totp_qr_svg: String,
    pub onboard_totp_secret: String,
    pub onboard_notice: Option<String>,
    pub onboard_error: Option<String>,
}
#[derive(Template)]
#[template(path = "devices_list.html")]
pub struct DevicesListView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub search_term: String,
    pub export_csv_href: String,
    pub archived_hide: bool,
    pub archived_only: bool,
    pub archived_all: bool,
    pub seen_recent: bool,
    pub empty_inventory: bool,
    pub filter_empty: bool,
    pub devices: Vec<DeviceRowView>,
}
#[derive(Clone)]
pub struct DeviceRowView {
    pub device_uuid: String,
    pub alias: String,
    pub site_display: String,
    pub tags_display: String,
    pub notes_display: String,
    pub notes_title: String,
    pub rustdesk_id_display: String,
    pub rustdesk_id_copy_text: String,
    pub default_helper_copy_text: String,
    pub explicit_helper_copy_text: String,
    pub hostname_display: String,
    pub last_checkin_display: String,
    pub recently_seen: bool,
    pub status_display: String,
    pub archived_display: String,
}
#[derive(Clone)]
pub struct TagOptionView {
    pub tag_uuid: String,
    pub name: String,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "tags.html")]
pub struct TagsListView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub tags: Vec<TagRowView>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct TagRowView {
    pub tag_uuid: String,
    pub name: String,
}

#[derive(Template)]
#[template(path = "sites.html")]
pub struct SitesListView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub sites: Vec<SiteRowView>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct SiteRowView {
    pub site_uuid: String,
    pub name: String,
}

#[derive(Clone)]
pub struct SiteOptionView {
    pub site_uuid: String,
    pub name: String,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "device_form.html")]
pub struct DeviceFormView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub heading: String,
    pub form_action: String,
    pub device_uuid: String,
    pub alias: String,
    pub rustdesk_id: String,
    pub show_rustdesk_id_copy: bool,
    pub default_helper_copy_text: String,
    pub explicit_helper_copy_text: String,
    pub hostname: String,
    pub owner: String,
    pub notes: String,
    pub site_options: Vec<SiteOptionView>,
    pub tag_options: Vec<TagOptionView>,
    pub error_message: Option<String>,
    pub show_archive_actions: bool,
    pub show_unarchive_actions: bool,
}

#[derive(Template)]
#[template(path = "backup.html")]
pub struct BackupView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub message: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "server_config.html")]
pub struct ServerConfigView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub id_server: String,
    pub relay_server: String,
    pub api_server: String,
    pub public_key: String,
    pub message: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct HealthCheckRowView {
    pub label: String,
    pub target: String,
    pub status: String,
    pub status_pill_class: String,
    pub detail: String,
}

#[derive(Template)]
#[template(path = "status.html")]
pub struct StatusView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub id_server: String,
    pub relay_server: String,
    pub public_key_fingerprint: String,
    pub backup_status: String,
    pub backup_status_pill_class: String,
    pub backup_execution: String,
    pub backup_execution_pill_class: String,
    pub checks: Vec<HealthCheckRowView>,
}

pub fn status_pill_class(status: &str) -> &'static str {
    match status {
        "ok" | "configured_external_runner" => "pill-ok",
        "failed" | "incomplete" | "error" | "degraded" => "pill-bad",
        _ => "pill-muted",
    }
}

#[derive(Clone)]
pub struct EnrollmentTokenOptionView {
    pub enrollment_token_uuid: String,
    pub label: String,
    pub status: String,
    pub selected: bool,
}

#[derive(Template)]
#[template(path = "enrollment_tokens.html")]
pub struct EnrollmentTokensView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub tokens: Vec<EnrollmentTokenRowView>,
    pub created_token_value: Option<String>,
    pub created_onboard_url: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct EnrollmentTokenRowView {
    pub enrollment_token_uuid: String,
    pub label: String,
    pub status: String,
    pub expires_display: String,
    pub can_revoke: bool,
}

#[derive(Template)]
#[template(path = "audit.html")]
pub struct AuditLogView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub events: Vec<AuditEventRowView>,
    pub tabs: Vec<AuditKindTabView>,
    pub export_csv_href: String,
    pub intro: String,
    pub connection_disclaimer: Option<String>,
    pub empty_message: String,
    pub has_events: bool,
}

#[derive(Clone)]
pub struct AuditKindTabView {
    pub label: String,
    pub href: String,
    pub selected: bool,
}

#[derive(Clone)]
pub struct AuditEventRowView {
    pub created_at: String,
    pub actor_display: String,
    pub action: String,
    pub object_type: String,
    pub object_uuid_display: String,
    pub outcome: String,
    pub source: String,
    pub detail_display: String,
    pub detail_title: String,
}

#[derive(Template)]
#[template(path = "users.html")]
pub struct UsersListView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub users: Vec<UserRowView>,
    pub error_message: Option<String>,
}
#[derive(Clone)]
pub struct UserRowView {
    pub user_uuid: String,
    pub username: String,
    pub role_display: String,
    pub activation_state: String,
    pub can_activate: bool,
    pub can_disable: bool,
}

#[path = "views_access.rs"]
mod views_access;
pub use views_access::*;
