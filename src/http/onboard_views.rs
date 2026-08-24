use crate::http::views::NavPermissions;
use askama::Template;

#[derive(Template)]
#[template(path = "onboard.html")]
pub struct OnboardView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub valid: bool,
    pub configured: bool,
    pub show_code_form: bool,
    pub heading: String,
    pub message: String,
    pub error_message: Option<String>,
    pub technician: String,
    pub windows_setup_href: String,
    pub windows_setup_available: bool,
    pub windows_command: String,
    pub linux_command: String,
    pub macos_command: String,
}

#[derive(Template)]
#[template(path = "onboard_setup.html")]
pub struct OnboardSetupView {
    pub title: String,
    pub show_nav: bool,
    pub nav: NavPermissions,
    pub csrf_token: String,
    pub username: String,
    pub onboard_totp_enrolled: bool,
    pub onboard_totp_qr_svg: String,
    pub onboard_totp_secret: String,
    pub onboard_notice: Option<String>,
    pub onboard_error: Option<String>,
}
