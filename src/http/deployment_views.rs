use super::views::NavPermissions;
use askama::Template;

#[derive(Template)]
#[template(path = "deployment.html")]
pub struct DeploymentView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub configured: bool,
    pub id_server: String,
    pub relay_server: String,
    pub api_server: String,
    pub public_key: String,
    pub rustdesk_config: String,
    pub rustdesk_qr_svg: String,
    pub downloads: Vec<DeploymentDownloadView>,
    pub public_base_url: String,
    pub linux_script: String,
    pub windows_script: String,
    pub macos_script: String,
    pub filename_custom_server: String,
}

#[derive(Clone)]
pub struct DeploymentDownloadView {
    pub platform: String,
    pub description: String,
    pub url: String,
}
