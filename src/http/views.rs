use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginView {
    pub title: String,
    pub show_nav: bool,
    pub error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "devices_list.html")]
pub struct DevicesListView {
    pub title: String,
    pub show_nav: bool,
    pub search_term: String,
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
    pub hostname_display: String,
    pub last_checkin_display: String,
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
    pub heading: String,
    pub form_action: String,
    pub device_uuid: String,
    pub alias: String,
    pub rustdesk_id: String,
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
#[template(path = "server_config.html")]
pub struct ServerConfigView {
    pub title: String,
    pub show_nav: bool,
    pub id_server: String,
    pub relay_server: String,
    pub api_server: String,
    pub public_key: String,
    pub message: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "deployment.html")]
pub struct DeploymentView {
    pub title: String,
    pub show_nav: bool,
    pub tokens: Vec<EnrollmentTokenOptionView>,
    pub enrollment_token_value: String,
    pub public_base_url: String,
    pub linux_script: String,
    pub windows_script: String,
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
    pub tokens: Vec<EnrollmentTokenRowView>,
    pub created_token_value: Option<String>,
}

#[derive(Clone)]
pub struct EnrollmentTokenRowView {
    pub enrollment_token_uuid: String,
    pub label: String,
    pub status: String,
    pub can_revoke: bool,
}