mod common;

use std::process::Command;
use std::{fs, time::Duration};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use common::{login_and_get_session_cookie, test_state};
use opendesk::build_router;
use serde_json::json;
use tokio::net::TcpListener;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn login_page_renders_opendesk_form() {
    let app = build_router(test_state().await);
    let response = app
        .oneshot(Request::builder().uri("/login").body(Body::empty()).unwrap())
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("OpenDesk"));
    assert!(html.contains("Admin Login"));
}

#[tokio::test]
async fn device_update_via_handler_preserves_enrollment_metadata() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "metadata-token",
        None,
        None,
        None,
    )
    .await
    .expect("create token");

    let app = build_router(state.clone());
    let checkin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/enrollments/check-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enrollment_token": created.token_value,
                        "rustdesk_id": "554433221",
                        "hostname": "dev-host",
                        "os_family": "linux",
                        "os_version": "test-os",
                        "architecture": "x86_64",
                        "rustdesk_version": "1.4.8"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("checkin");
    assert_eq!(checkin.status(), StatusCode::NO_CONTENT);

    let device = opendesk::repository::devices::find_device_by_rustdesk_id(
        &state.db,
        "554433221",
    )
    .await
    .expect("lookup")
    .expect("device");

    let session_cookie = login_and_get_session_cookie(&app).await;
    let update = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/devices/{}", device.device_uuid))
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie)
                .body(Body::from(
                    "alias=Renamed+device&notes=operator+note&rustdesk_id=&hostname=&owner=",
                ))
                .unwrap(),
        )
        .await
        .expect("device update");
    assert_eq!(update.status(), StatusCode::SEE_OTHER);

    let updated = opendesk::repository::devices::find_device_by_uuid(
        &state.db,
        device.device_uuid,
    )
    .await
    .expect("reload")
    .expect("updated device");

    assert_eq!(updated.alias, "Renamed device");
    assert_eq!(updated.os_family.as_deref(), Some("linux"));
    assert_eq!(updated.architecture.as_deref(), Some("x86_64"));
    assert_eq!(updated.rustdesk_version.as_deref(), Some("1.4.8"));
    assert_eq!(updated.notes.as_deref(), Some("operator note"));
}

#[tokio::test]
async fn linux_script_export_executes_check_in_against_running_server() {
    let state = test_state().await;
    let created = opendesk::repository::enrollment_tokens::create_enrollment_token(
        &state.db,
        "script-export-token",
        None,
        None,
        None,
    )
    .await
    .expect("create token");

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind listener");
    let addr = listener.local_addr().expect("listener address");
    let mut state = state;
    state.public_base_url = format!("http://{addr}");
    let app = build_router(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve integration test app");
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let session_cookie = login_and_get_session_cookie(&build_router(state.clone())).await;
    let export_uri = format!(
        "/deployment/linux.sh?enrollment_token_value={}",
        created.token_value
    );
    let response = build_router(state.clone())
        .oneshot(
            Request::builder()
                .uri(export_uri)
                .header("cookie", session_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("linux script export");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let script = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(script.contains("#!/usr/bin/env bash"));
    assert!(script.contains(&created.token_value));
    assert!(script.contains("opendesk enrollment check-in http_status="));

    let temp_root = std::env::temp_dir().join(format!(
        "opendesk-linux-script-test-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&temp_root).expect("temp dir");
    let fake_bin = temp_root.join("bin");
    fs::create_dir_all(&fake_bin).expect("fake bin dir");
    let fake_rustdesk = fake_bin.join("rustdesk");
    fs::write(
        &fake_rustdesk,
        r#"#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --get-id)
    echo "887766554"
    ;;
  --option)
    exit 0
    ;;
  --get-option)
    echo "rd.example.com"
    ;;
  *)
    exit 0
    ;;
esac
"#,
    )
    .expect("write fake rustdesk");
    #[cfg(unix)]
    fs::set_permissions(&fake_rustdesk, fs::Permissions::from_mode(0o755))
        .expect("chmod fake rustdesk");

    let script_path = temp_root.join("deploy.sh");
    fs::write(&script_path, &script).expect("write exported script");
    #[cfg(unix)]
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))
        .expect("chmod exported script");

    let path_for_script = script_path.clone();
    let path_for_bin = fake_bin.clone();
    let output = tokio::task::spawn_blocking(move || {
        Command::new("bash")
            .arg(path_for_script)
            .env("PATH", format!("{}:/usr/bin:/bin", path_for_bin.display()))
            .output()
    })
    .await
    .expect("join script execution task")
    .expect("execute exported script");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "script failed stdout={stdout} stderr={stderr}"
    );
    assert!(stdout.contains("opendesk enrollment check-in http_status=204"));

    let device = opendesk::repository::devices::find_device_by_rustdesk_id(
        &state.db,
        "887766554",
    )
    .await
    .expect("lookup enrolled device")
    .expect("device created by exported script check-in");
    assert_eq!(device.os_family.as_deref(), Some("linux"));

    let _ = fs::remove_dir_all(&temp_root);
}

#[tokio::test]
async fn archived_device_validation_error_shows_unarchive_action() {
    let state = test_state().await;
    let device = opendesk::repository::devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Archive test".to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("create device");
    opendesk::repository::devices::set_device_archived(&state.db, device.device_uuid, true)
        .await
        .expect("archive");

    let app = build_router(state);
    let session_cookie = login_and_get_session_cookie(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/devices/{}", device.device_uuid))
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", session_cookie)
                .body(Body::from("alias=+&rustdesk_id=&hostname=&owner=&notes="))
                .unwrap(),
        )
        .await
        .expect("validation error response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Unarchive"));
    assert!(!html.contains("/archive\">\n    <button type=\"submit\">Archive</button>"));
}

