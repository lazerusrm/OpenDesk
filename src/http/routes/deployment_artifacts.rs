use std::path::{Path, PathBuf};

use axum::{
    body::Body,
    extract::{path::Path as AxumPath, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::app_state::AppState;
use crate::config::{nofollow_regular_file_exists, read_nofollow_file};
use crate::domain::access_policy::Action;
use crate::http::deployment_views::FirstPartyDownloadView;
use crate::http::session::require_action;

const WINDOWS_SETUP_EXE: Artifact = Artifact {
    disk_name: "windows-setup.exe",
    route_name: "setup.exe",
    href: "/deployment/windows/setup.exe",
    label: "Download signed Windows installer",
    content_type: "application/vnd.microsoft.portable-executable",
};
const WINDOWS_SETUP_MSI: Artifact = Artifact {
    disk_name: "windows-setup.msi",
    route_name: "setup.msi",
    href: "/deployment/windows/setup.msi",
    label: "Download signed Windows package",
    content_type: "application/x-msi",
};
const WINDOWS_ARTIFACTS: &[Artifact] = &[WINDOWS_SETUP_EXE, WINDOWS_SETUP_MSI];

#[derive(Clone, Copy)]
struct Artifact {
    disk_name: &'static str,
    route_name: &'static str,
    href: &'static str,
    label: &'static str,
    content_type: &'static str,
}

pub fn windows_downloads(state: &AppState) -> Vec<FirstPartyDownloadView> {
    WINDOWS_ARTIFACTS
        .iter()
        .filter(|artifact| resolve_artifact(state.signed_client_dir.as_deref(), artifact).is_some())
        .map(|artifact| FirstPartyDownloadView {
            href: artifact.href.to_string(),
            label: artifact.label.to_string(),
        })
        .collect()
}

pub(super) async fn windows_setup_file(
    State(state): State<AppState>,
    jar: CookieJar,
    AxumPath(name): AxumPath<String>,
) -> Result<Response, Response> {
    let user = require_action(&state, &jar, Action::DeploymentView).await?;
    let artifact = WINDOWS_ARTIFACTS
        .iter()
        .find(|candidate| candidate.route_name == name)
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let path = resolve_artifact(state.signed_client_dir.as_deref(), artifact)
        .ok_or_else(|| StatusCode::NOT_FOUND.into_response())?;
    let bytes = read_nofollow_file(&path).map_err(|_| StatusCode::NOT_FOUND.into_response())?;
    super::write_deployment_audit(
        &state,
        &user,
        "deployment_artifact_download",
        serde_json::json!({ "artifact": artifact.disk_name }),
    )
    .await;
    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(artifact.content_type),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", artifact.disk_name))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    Ok(response)
}

fn resolve_artifact(root: Option<&Path>, artifact: &Artifact) -> Option<PathBuf> {
    let root = root?;
    if artifact.disk_name.contains('/') || artifact.disk_name.contains('\\') {
        return None;
    }
    let path = root.join(artifact.disk_name);
    if !nofollow_regular_file_exists(&path) {
        return None;
    }
    Some(path)
}
