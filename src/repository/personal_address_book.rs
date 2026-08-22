use std::collections::HashSet;

use sqlx::SqlitePool;
use uuid::Uuid;

use super::{
    create_address_book_entry, create_personal_address_book, find_personal_address_book,
    map_sql_error, parse_uuid, AddressBookRepositoryError,
};
use crate::domain::address_book::AddressBook;
use crate::repository::device_visibility::list_explicitly_granted_device_uuids_for_user;
use crate::repository::devices::list_devices;

pub async fn ensure_personal_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
) -> Result<AddressBook, AddressBookRepositoryError> {
    match find_personal_address_book(pool, owner_user_uuid).await {
        Ok(book) => Ok(book),
        Err(AddressBookRepositoryError::NotFound) => {
            match create_personal_address_book(pool, owner_user_uuid, "Personal").await {
                Ok(book) => Ok(book),
                Err(AddressBookRepositoryError::Conflict) => {
                    match find_personal_address_book(pool, owner_user_uuid).await {
                        Ok(book) => Ok(book),
                        Err(AddressBookRepositoryError::NotFound) => {
                            create_personal_address_book(pool, owner_user_uuid, "My address book")
                                .await
                        }
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

pub async fn sync_visible_devices_into_personal_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let visible: HashSet<Uuid> =
        list_explicitly_granted_device_uuids_for_user(pool, owner_user_uuid)
            .await
            .map_err(map_sql_error)?
            .into_iter()
            .collect();
    let hidden: HashSet<Uuid> = sqlx::query_scalar::<_, String>(
        "SELECT device_uuid FROM personal_address_book_hidden_devices WHERE owner_user_uuid = ?",
    )
    .bind(owner_user_uuid.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql_error)?
    .into_iter()
    .map(parse_uuid)
    .collect();
    let existing: HashSet<Uuid> = sqlx::query_scalar::<_, String>(
        "SELECT device_uuid FROM address_book_entries WHERE address_book_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql_error)?
    .into_iter()
    .map(parse_uuid)
    .collect();
    let max_position: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(position), -1) FROM address_book_entries WHERE address_book_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .fetch_one(pool)
    .await
    .map_err(map_sql_error)?;
    let mut next_position = u32::try_from(max_position.saturating_add(1)).unwrap_or(0);
    let devices = list_devices(pool).await.map_err(map_sql_error)?;
    for device in devices {
        let rustdesk_id = device
            .rustdesk_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if device.archived
            || rustdesk_id.is_none()
            || !visible.contains(&device.device_uuid)
            || hidden.contains(&device.device_uuid)
            || existing.contains(&device.device_uuid)
        {
            continue;
        }
        match create_address_book_entry(
            pool,
            owner_user_uuid,
            address_book_uuid,
            device.device_uuid,
            &device.alias,
            device.notes.clone(),
            next_position,
        )
        .await
        {
            Ok(_) => next_position = next_position.saturating_add(1),
            Err(AddressBookRepositoryError::Conflict) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub async fn hide_personal_address_book_device(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    sqlx::query(
        "INSERT OR IGNORE INTO personal_address_book_hidden_devices
         (owner_user_uuid, device_uuid)
         SELECT ?, ?
         FROM address_books
         WHERE address_book_uuid = ?
           AND owner_user_uuid = ?
           AND book_kind = 'personal'",
    )
    .bind(owner_user_uuid.to_string())
    .bind(device_uuid.to_string())
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    Ok(())
}
