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
}

#[derive(Clone)]
pub struct DeploymentDownloadView {
    pub platform: String,
    pub description: String,
    pub page_href: String,
}

#[derive(Template)]
#[template(path = "deployment_os.html")]
pub struct DeploymentOsView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub configured: bool,
    pub platform: String,
    pub description: String,
    pub official_download_url: String,
    pub id_server: String,
    pub relay_server: String,
    pub api_server: String,
    pub public_key: String,
    pub rustdesk_config: String,
    pub rustdesk_qr_svg: String,
    pub public_base_url: String,
    pub show_script: bool,
    pub script_heading: String,
    pub script: String,
    pub script_download_href: String,
    pub script_download_label: String,
    pub show_filename_fallback: bool,
    pub filename_custom_server: String,
    pub is_android: bool,
    pub is_ios: bool,
    pub first_party_windows_downloads: Vec<FirstPartyDownloadView>,
}

#[derive(Clone)]
pub struct FirstPartyDownloadView {
    pub href: String,
    pub label: String,
}
