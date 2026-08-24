use crate::app_state::AppState;

pub(super) const RUSTDESK_RELEASES_URL: &str = "https://github.com/rustdesk/rustdesk/releases";

#[derive(Clone, Copy)]
pub(super) enum DeploymentOs {
    Windows,
    Linux,
    Macos,
    Android,
    Ios,
}

impl DeploymentOs {
    pub(super) fn platform(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::Macos => "macOS",
            Self::Android => "Android",
            Self::Ios => "iOS",
        }
    }

    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Windows => {
                "Official desktop packages, config import, and generated PowerShell script."
            }
            Self::Linux => {
                "Official distribution packages, config import, and generated shell script."
            }
            Self::Macos => "Official Apple packages, config import, and generated shell script.",
            Self::Android => "Official mobile app with QR and manual ID, relay, and key setup.",
            Self::Ios => "Official operator app. iOS cannot be a controlled endpoint.",
        }
    }

    pub(super) fn page_href(self) -> &'static str {
        match self {
            Self::Windows => "/deployment/windows",
            Self::Linux => "/deployment/linux",
            Self::Macos => "/deployment/macos",
            Self::Android => "/deployment/android",
            Self::Ios => "/deployment/ios",
        }
    }

    pub(super) fn official_download_url(self, state: &AppState) -> &str {
        let configured = match self {
            Self::Windows => state.rustdesk_download_windows_url.as_deref(),
            Self::Macos => state.rustdesk_download_macos_url.as_deref(),
            Self::Linux => state.rustdesk_download_linux_url.as_deref(),
            Self::Android => state.rustdesk_download_android_url.as_deref(),
            Self::Ios => None,
        };
        configured.unwrap_or(RUSTDESK_RELEASES_URL)
    }

    pub(super) fn script_export(self) -> Option<(&'static str, &'static str, &'static str)> {
        match self {
            Self::Windows => Some((
                "/deployment/windows.ps1",
                "Download Windows script",
                "Windows PowerShell script",
            )),
            Self::Linux => Some((
                "/deployment/linux.sh",
                "Download Linux script",
                "Linux shell script",
            )),
            Self::Macos => Some((
                "/deployment/macos.sh",
                "Download macOS script",
                "macOS shell script",
            )),
            Self::Android | Self::Ios => None,
        }
    }
}
