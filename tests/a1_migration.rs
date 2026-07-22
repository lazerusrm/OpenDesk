mod common;

use common::test_state;
use sqlx::Row;
use uuid::Uuid;

#[tokio::test]
async fn a1_migration_enforces_visibility_and_address_book_constraints() {
    let state = test_state().await;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&state.db)
        .await
        .expect("enable foreign keys");

    let user_uuid = Uuid::new_v4().to_string();
    let device_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (user_uuid, username, password_hash, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_uuid)
    .bind("a1-user")
    .bind("hash")
    .bind("admin")
    .bind("now")
    .bind("now")
    .execute(&state.db)
    .await
    .expect("user");
    sqlx::query(
        "INSERT INTO devices (device_uuid, alias, archived, created_at, updated_at)
         VALUES (?, ?, 0, ?, ?)",
    )
    .bind(&device_uuid)
    .bind("A1 device")
    .bind("now")
    .bind("now")
    .execute(&state.db)
    .await
    .expect("device");

    let group_uuid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, ?)")
        .bind(&group_uuid)
        .bind("Operators")
        .execute(&state.db)
        .await
        .expect("group");
    sqlx::query(
        "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
    )
    .bind(&group_uuid)
    .bind(&user_uuid)
    .execute(&state.db)
    .await
    .expect("membership");
    sqlx::query(
        "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid) VALUES (?, ?)",
    )
    .bind(&group_uuid)
    .bind(&device_uuid)
    .execute(&state.db)
    .await
    .expect("group grant");
    sqlx::query("INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)")
        .bind(&user_uuid)
        .bind(&device_uuid)
        .execute(&state.db)
        .await
        .expect("direct grant");

    let book_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name) VALUES (?, ?, ?)",
    )
    .bind(&book_uuid)
    .bind(&user_uuid)
    .bind("Favorites")
    .execute(&state.db)
    .await
    .expect("book");
    let entry_uuid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO address_book_entries
         (address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry_uuid)
    .bind(&book_uuid)
    .bind(&device_uuid)
    .bind("A1 device")
    .bind("notes")
    .bind(0_i64)
    .execute(&state.db)
    .await
    .expect("entry");

    let duplicate = sqlx::query(
        "INSERT INTO address_book_entries
         (address_book_entry_uuid, address_book_uuid, device_uuid, alias, position)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&book_uuid)
    .bind(&device_uuid)
    .bind("duplicate")
    .bind(1_i64)
    .execute(&state.db)
    .await;
    assert!(duplicate.is_err(), "book/device pair must be unique");

    let invalid_position = sqlx::query(
        "INSERT INTO address_book_entries
         (address_book_entry_uuid, address_book_uuid, device_uuid, alias, position)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&book_uuid)
    .bind(Uuid::new_v4().to_string())
    .bind("negative")
    .bind(-1_i64)
    .execute(&state.db)
    .await;
    assert!(invalid_position.is_err(), "position must be nonnegative");

    let duplicate_membership = sqlx::query(
        "INSERT INTO access_group_memberships (access_group_uuid, user_uuid) VALUES (?, ?)",
    )
    .bind(&group_uuid)
    .bind(&user_uuid)
    .execute(&state.db)
    .await;
    assert!(
        duplicate_membership.is_err(),
        "membership pair must be unique"
    );

    let dangling_grant = sqlx::query(
        "INSERT INTO user_device_visibility_grants (user_uuid, device_uuid) VALUES (?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&device_uuid)
    .execute(&state.db)
    .await;
    assert!(dangling_grant.is_err(), "grant references must be enforced");

    let duplicate_book = sqlx::query(
        "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name) VALUES (?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user_uuid)
    .bind("Favorites")
    .execute(&state.db)
    .await;
    assert!(duplicate_book.is_err(), "owner/book name must be unique");

    let book_count: i64 = sqlx::query("SELECT COUNT(*) AS count FROM address_books")
        .fetch_one(&state.db)
        .await
        .expect("book count")
        .get("count");
    assert_eq!(book_count, 1);

    let group_count: i64 = sqlx::query("SELECT COUNT(*) AS count FROM access_groups")
        .fetch_one(&state.db)
        .await
        .expect("count")
        .get("count");
    assert_eq!(group_count, 1);
}
