mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{form_with_csrf, login_and_get_session_cookie, test_state};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::repository::{address_books, device_visibility, devices, users};
use tower::ServiceExt;

async fn get_html(app: &axum::Router, cookie: &str, uri: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("page");
    assert_eq!(response.status(), StatusCode::OK);
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}

#[tokio::test]
async fn address_books_list_empty_state_includes_create_form_and_disclaimer() {
    let state = test_state().await;
    let app = build_router(state);
    let cookie = login_and_get_session_cookie(&app).await;
    let html = get_html(&app, &cookie, "/address-books").await;
    assert!(html.contains("No address books"));
    assert!(html.contains("Create address book"));
    assert!(html.contains("This web address book is OpenDesk data"));
    assert!(html.contains("Official clients use isolated /api/ab* compatibility"));
    assert!(html.contains("user or group"));
    assert!(html.contains("read, write, or admin"));
    assert!(html.contains("Shared with you"));
    assert!(html.contains("Nothing has been shared with you yet"));
    assert!(!html.contains("this console does not edit those grants"));
    assert!(!html.contains("<code>"));
}

#[tokio::test]
async fn address_book_detail_empty_entries_picker_notes_and_rustdesk_id_copy() {
    let state = test_state().await;
    let admin = users::list_users(&state.db).await.expect("users").remove(0);
    let app = build_router(state.clone());
    let cookie = login_and_get_session_cookie(&app).await;

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/address-books")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "name=Favorites&book_kind=personal",
                )))
                .unwrap(),
        )
        .await
        .expect("personal book");
    assert_eq!(created.status(), StatusCode::SEE_OTHER);
    let book = address_books::list_address_books_for_owner(&state.db, admin.user_uuid)
        .await
        .expect("books")
        .into_iter()
        .next()
        .expect("book");

    let empty_detail = get_html(
        &app,
        &cookie,
        &format!("/address-books/{}", book.address_book_uuid),
    )
    .await;
    assert!(empty_detail.contains("No entries"));
    assert!(empty_detail.contains("Kind: Personal"));
    assert!(empty_detail.contains("No visible devices available to add."));
    assert!(empty_detail.contains("This web address book is OpenDesk data"));
    assert!(!empty_detail.contains("<code>"));
    assert!(!empty_detail.contains("Address book UUID"));

    let device = devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Copy Bench".into(),
            rustdesk_id: Some("123456789".into()),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        admin.user_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("visibility");

    let picker = get_html(
        &app,
        &cookie,
        &format!("/address-books/{}", book.address_book_uuid),
    )
    .await;
    assert!(picker.contains("Copy Bench · 123456789"));
    assert!(picker.contains("Optional notes"));
    assert!(!picker.contains("No visible devices available to add."));

    let added = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/address-books/{}/entries", book.address_book_uuid))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    &format!(
                        "device_uuid={}&alias=Bench&notes=Keep this peer labeled&position=1",
                        device.device_uuid
                    ),
                )))
                .unwrap(),
        )
        .await
        .expect("entry");
    assert_eq!(added.status(), StatusCode::SEE_OTHER);

    let detail = get_html(
        &app,
        &cookie,
        &format!("/address-books/{}", book.address_book_uuid),
    )
    .await;
    assert!(detail.contains("Bench"));
    assert!(detail.contains("Copy Bench"));
    assert!(detail.contains("Keep this peer labeled"));
    assert!(detail.contains(r#"data-copy-text="123456789""#));
    assert!(detail.contains("Copy ID"));
    assert!(detail.contains("Save entry"));
    assert!(detail.contains("No visible devices available to add."));
    assert!(!detail.contains("No entries"));
    assert!(!detail.contains("<code>"));

    let shared = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/address-books")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &cookie,
                    "name=Team&book_kind=shared",
                )))
                .unwrap(),
        )
        .await
        .expect("shared book");
    assert_eq!(shared.status(), StatusCode::SEE_OTHER);

    let list = get_html(&app, &cookie, "/address-books").await;
    assert!(!list.contains("No address books"));
    assert!(list.contains("Favorites"));
    assert!(list.contains("Team"));
    assert!(list.contains("Personal"));
    assert!(list.contains("Shared"));
    assert!(list.contains("Shared with you"));
    assert!(!list.contains("this console does not edit those grants"));
}
