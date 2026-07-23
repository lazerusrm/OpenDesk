mod common;

use common::test_state;
use opendesk::domain::device::DeviceDraft;
use opendesk::repository::{access_groups, address_books, device_visibility, devices, users};
use uuid::Uuid;

async fn create_device(state: &opendesk::AppState, alias: &str) -> Uuid {
    devices::create_device(
        &state.db,
        &DeviceDraft {
            alias: alias.to_string(),
            ..Default::default()
        },
    )
    .await
    .expect("device")
    .device_uuid
}

#[tokio::test]
async fn group_replacements_are_atomic_and_visibility_is_default_deny() {
    let state = test_state().await;
    let admin = users::find_user_by_username(&state.db, "admin")
        .await
        .expect("lookup admin")
        .expect("admin");
    let operator = users::create_user(&state.db, "operator", "password", "operator")
        .await
        .expect("operator");
    let device_uuid = create_device(&state, "Visible device").await;
    let unrelated_device_uuid = create_device(&state, "Other device").await;
    let group = access_groups::create_access_group(&state.db, "Operators")
        .await
        .expect("group");

    assert!(!device_visibility::is_device_visible_to_user(
        &state.db,
        operator.user_uuid,
        device_uuid
    )
    .await
    .expect("default deny"));
    access_groups::replace_access_group_memberships(
        &state.db,
        group.access_group_uuid,
        &[operator.user_uuid],
    )
    .await
    .expect("membership");
    access_groups::replace_group_device_visibility_grants(
        &state.db,
        group.access_group_uuid,
        &[device_uuid],
    )
    .await
    .expect("group grant");
    assert!(device_visibility::is_device_visible_to_user(
        &state.db,
        operator.user_uuid,
        device_uuid
    )
    .await
    .expect("group visibility"));
    assert!(!device_visibility::is_device_visible_to_user(
        &state.db,
        operator.user_uuid,
        unrelated_device_uuid
    )
    .await
    .expect("unrelated device denied"));

    let missing_user = Uuid::new_v4();
    assert!(access_groups::replace_access_group_memberships(
        &state.db,
        group.access_group_uuid,
        &[missing_user]
    )
    .await
    .is_err());
    assert_eq!(
        access_groups::list_access_group_memberships(&state.db, group.access_group_uuid)
            .await
            .expect("memberships")
            .into_iter()
            .map(|membership| membership.user_uuid)
            .collect::<Vec<_>>(),
        vec![operator.user_uuid]
    );
    assert!(access_groups::replace_group_device_visibility_grants(
        &state.db,
        group.access_group_uuid,
        &[device_uuid, device_uuid]
    )
    .await
    .is_err());

    access_groups::replace_access_group_memberships(&state.db, group.access_group_uuid, &[])
        .await
        .expect("clear memberships");
    assert!(!device_visibility::is_device_visible_to_user(
        &state.db,
        operator.user_uuid,
        device_uuid
    )
    .await
    .expect("membership removal denies visibility"));
    assert!(
        !device_visibility::is_device_visible_to_user(&state.db, admin.user_uuid, device_uuid)
            .await
            .expect("admin role does not grant visibility")
    );
}

#[tokio::test]
async fn direct_visibility_replacement_validates_inside_transaction() {
    let state = test_state().await;
    let user = users::create_user(&state.db, "direct", "password", "operator")
        .await
        .expect("user");
    let device_uuid = create_device(&state, "Direct device").await;
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        user.user_uuid,
        &[device_uuid],
    )
    .await
    .expect("direct grant");
    assert!(device_visibility::replace_user_device_visibility_grants(
        &state.db,
        user.user_uuid,
        &[Uuid::new_v4()]
    )
    .await
    .is_err());
    assert_eq!(
        device_visibility::list_visible_device_uuids_for_user(&state.db, user.user_uuid)
            .await
            .expect("visible devices"),
        vec![device_uuid]
    );
}

#[tokio::test]
async fn address_books_are_owner_scoped_and_entries_require_real_devices() {
    let state = test_state().await;
    let owner = users::create_user(&state.db, "book-owner", "password", "operator")
        .await
        .expect("owner");
    let other_owner = users::create_user(&state.db, "other-owner", "password", "operator")
        .await
        .expect("other owner");
    let device_uuid = create_device(&state, "Book device").await;
    let hidden_device_uuid = create_device(&state, "Hidden device").await;
    device_visibility::replace_user_device_visibility_grants(
        &state.db,
        owner.user_uuid,
        &[device_uuid],
    )
    .await
    .expect("owner visibility");
    let book =
        address_books::create_personal_address_book(&state.db, owner.user_uuid, " Favorites ")
            .await
            .expect("book");
    assert_eq!(book.name, "Favorites");
    assert!(
        address_books::list_address_books_for_owner(&state.db, other_owner.user_uuid)
            .await
            .expect("other books")
            .is_empty()
    );
    assert!(address_books::create_address_book_entry(
        &state.db,
        owner.user_uuid,
        book.address_book_uuid,
        hidden_device_uuid,
        "Hidden",
        None,
        0,
    )
    .await
    .is_err());
    let entry = address_books::create_address_book_entry(
        &state.db,
        owner.user_uuid,
        book.address_book_uuid,
        device_uuid,
        " Workstation ",
        Some(" notes ".to_string()),
        0,
    )
    .await
    .expect("entry");
    assert_eq!(entry.alias, "Workstation");
    assert!(matches!(
        address_books::list_address_book_entries(
            &state.db,
            other_owner.user_uuid,
            book.address_book_uuid
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    assert!(matches!(
        address_books::find_address_book_for_owner(
            &state.db,
            other_owner.user_uuid,
            book.address_book_uuid,
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    assert!(matches!(
        address_books::find_address_book_for_owner(&state.db, owner.user_uuid, Uuid::new_v4(),)
            .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    assert!(matches!(
        address_books::delete_address_book(
            &state.db,
            other_owner.user_uuid,
            book.address_book_uuid,
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    assert!(matches!(
        address_books::rename_address_book(
            &state.db,
            other_owner.user_uuid,
            book.address_book_uuid,
            "Hijack",
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));

    assert!(matches!(
        address_books::update_address_book_entry(
            &state.db,
            owner.user_uuid,
            entry.address_book_entry_uuid,
            hidden_device_uuid,
            "Revoked",
            None,
            0,
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    device_visibility::replace_user_device_visibility_grants(&state.db, owner.user_uuid, &[])
        .await
        .expect("revoke owner visibility");
    assert!(address_books::list_address_book_entries(
        &state.db,
        owner.user_uuid,
        book.address_book_uuid,
    )
    .await
    .expect("hidden entries filtered")
    .is_empty());
    assert!(matches!(
        address_books::update_address_book_entry(
            &state.db,
            other_owner.user_uuid,
            entry.address_book_entry_uuid,
            device_uuid,
            "Hijack",
            None,
            0,
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    assert!(matches!(
        address_books::update_address_book_entry(
            &state.db,
            owner.user_uuid,
            Uuid::new_v4(),
            device_uuid,
            "Missing",
            None,
            0,
        )
        .await,
        Err(address_books::AddressBookRepositoryError::NotFound)
    ));
    address_books::delete_address_book_entry(
        &state.db,
        owner.user_uuid,
        entry.address_book_entry_uuid,
    )
    .await
    .expect("owner deletes entry");
}
