mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    form_with_csrf, login_and_get_session_cookie, session_cookie_from_response, test_state,
};
use http_body_util::BodyExt;
use opendesk::build_router;
use opendesk::domain::role::Role;
use opendesk::repository::{access_groups, address_books, device_visibility, devices, users};
use tower::ServiceExt;
use uuid::Uuid;

async fn login_as(app: &axum::Router, username: &str, password: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/login")
                .header("origin", "http://127.0.0.1:8080")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "username={username}&password={password}"
                )))
                .unwrap(),
        )
        .await
        .expect("login");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    session_cookie_from_response(&response)
}

#[tokio::test]
async fn access_group_pages_are_admin_only_and_replacements_prg() {
    let state = test_state().await;
    let operator = users::create_user(&state.db, "operator", "operator-password", Role::READ_ONLY)
        .await
        .expect("operator");
    let device = devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Visible device".into(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let app = build_router(state.clone());
    let admin_cookie = login_and_get_session_cookie(&app).await;

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/access-groups")
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(&admin_cookie, "name=Operators")))
                .unwrap(),
        )
        .await
        .expect("create group");
    assert_eq!(created.status(), StatusCode::SEE_OTHER);
    let groups = access_groups::list_access_groups(&state.db)
        .await
        .expect("groups");
    let group = groups.first().expect("group");

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/access-groups/{}", group.access_group_uuid))
                .header("cookie", &admin_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("detail");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_html = String::from_utf8(
        detail
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(detail_html.contains("Operators"));
    assert!(!detail_html.contains("Access group UUID"));
    assert!(!detail_html.contains("<code>"));

    let membership_body =
        form_with_csrf(&admin_cookie, &format!("user_uuid={}", operator.user_uuid));
    let membership = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/access-groups/{}/memberships",
                    group.access_group_uuid
                ))
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(membership_body))
                .unwrap(),
        )
        .await
        .expect("memberships");
    assert_eq!(membership.status(), StatusCode::SEE_OTHER);

    let visibility = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/access-groups/{}/device-visibility-grants",
                    group.access_group_uuid
                ))
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_with_csrf(
                    &admin_cookie,
                    &format!("device_uuid={}", device.device_uuid),
                )))
                .unwrap(),
        )
        .await
        .expect("visibility");
    assert_eq!(visibility.status(), StatusCode::SEE_OTHER);

    let operator_cookie = login_as(&app, "operator", "operator-password").await;
    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/access-groups")
                .header("cookie", &operator_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("operator group page");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let bad_csrf = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/access-groups/{}/memberships",
                    group.access_group_uuid
                ))
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("csrf_token=wrong"))
                .unwrap(),
        )
        .await
        .expect("csrf");
    assert_eq!(bad_csrf.status(), StatusCode::FORBIDDEN);

    let unknown_group = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/access-groups/{}/memberships", Uuid::new_v4()))
                .header("cookie", &admin_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("csrf_token=wrong"))
                .unwrap(),
        )
        .await
        .expect("unknown group");
    assert_eq!(unknown_group.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn address_books_are_owner_scoped_and_entry_posts_prg() {
    let state = test_state().await;
    let device = devices::create_device(
        &state.db,
        &opendesk::domain::device::DeviceDraft {
            alias: "Book device".into(),
            ..Default::default()
        },
    )
    .await
    .expect("device");
    let admin = users::list_users(&state.db).await.expect("users").remove(0);
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        admin.user_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("visibility");
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
        .expect("book");
    assert_eq!(created.status(), StatusCode::SEE_OTHER);
    let book = address_books::list_address_books_for_owner(&state.db, admin.user_uuid)
        .await
        .expect("books")[0]
        .clone();

    let entry = app
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
                        "device_uuid={}&alias=Primary&notes=Work&position=0",
                        device.device_uuid
                    ),
                )))
                .unwrap(),
        )
        .await
        .expect("entry");
    assert_eq!(entry.status(), StatusCode::SEE_OTHER);

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/address-books/{}", book.address_book_uuid))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("book page");
    assert_eq!(page.status(), StatusCode::OK);
    let html = String::from_utf8(
        page.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("Primary"));
    assert!(html.contains("Save entry"));
    assert!(html.contains("Delete entry"));
    assert!(!html.contains("Address book UUID"));
    assert!(!html.contains("<code>"));

    let readonly = users::create_user(&state.db, "book-reader", "reader-password", Role::READ_ONLY)
        .await
        .expect("reader");
    let readonly_cookie = login_as(&app, "book-reader", "reader-password").await;
    let readonly_book =
        address_books::create_personal_address_book(&state.db, readonly.user_uuid, "Reader book")
            .await
            .expect("reader book");
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        readonly.user_uuid,
        &[device.device_uuid],
    )
    .await
    .expect("reader visibility");
    address_books::create_address_book_entry(
        &state.db,
        readonly.user_uuid,
        readonly_book.address_book_uuid,
        device.device_uuid,
        "Reader entry",
        None,
        0,
    )
    .await
    .expect("reader entry");
    let readonly_page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/address-books/{}",
                    readonly_book.address_book_uuid
                ))
                .header("cookie", &readonly_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("readonly page");
    assert_eq!(readonly_page.status(), StatusCode::OK);
    let readonly_html = String::from_utf8(
        readonly_page
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(!readonly_html.contains("Create address book"));
    assert!(!readonly_html.contains("Add entry"));
    assert!(!readonly_html.contains("Save entry"));
    assert!(!readonly_html.contains("Delete entry"));
    let _ = readonly;

    let foreign =
        address_books::create_personal_address_book(&state.db, Uuid::new_v4(), "No owner").await;
    assert!(foreign.is_err());
    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/address-books/{}", Uuid::new_v4()))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("missing");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
