use std::collections::HashSet;

use sqlx::SqlitePool;
use uuid::Uuid;

use super::{map_sql_error, parse_uuid, require_owned_book, AddressBookRepositoryError};
use crate::domain::address_book::AddressBookEntry;
use crate::repository::device_visibility::{
    is_device_visible_to_user, list_visible_device_uuids_for_user,
};

fn entry_from_row(
    (entry, book, device, alias, notes, position): (
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
    ),
) -> AddressBookEntry {
    AddressBookEntry {
        address_book_entry_uuid: parse_uuid(entry),
        address_book_uuid: parse_uuid(book),
        device_uuid: parse_uuid(device),
        alias,
        notes,
        position: u32::try_from(position).expect("stored nonnegative position"),
    }
}

async fn require_visible_device(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let visible = is_device_visible_to_user(pool, owner_user_uuid, device_uuid)
        .await
        .map_err(map_sql_error)?;
    if visible {
        Ok(())
    } else {
        Err(AddressBookRepositoryError::NotFound)
    }
}

pub async fn list_address_book_entries(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<Vec<AddressBookEntry>, AddressBookRepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_owned_book(&mut tx, address_book_uuid, owner_user_uuid).await?;
    let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, i64)>(
        "SELECT entry.address_book_entry_uuid, entry.address_book_uuid, entry.device_uuid,
                entry.alias, entry.notes, entry.position
         FROM address_book_entries entry
         WHERE entry.address_book_uuid = ?
         ORDER BY entry.position ASC, entry.address_book_entry_uuid ASC",
    )
    .bind(address_book_uuid.to_string())
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)?;
    let visible: HashSet<Uuid> = list_visible_device_uuids_for_user(pool, owner_user_uuid)
        .await
        .map_err(map_sql_error)?
        .into_iter()
        .collect();
    Ok(rows
        .into_iter()
        .map(entry_from_row)
        .filter(|entry| visible.contains(&entry.device_uuid))
        .collect())
}

pub async fn find_address_book_entry(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_entry_uuid: Uuid,
) -> Result<AddressBookEntry, AddressBookRepositoryError> {
    let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, i64)>(
        "SELECT entry.address_book_entry_uuid, entry.address_book_uuid, entry.device_uuid,
                entry.alias, entry.notes, entry.position
         FROM address_book_entries entry
         INNER JOIN address_books book ON book.address_book_uuid = entry.address_book_uuid
         WHERE entry.address_book_entry_uuid = ? AND book.owner_user_uuid = ?",
    )
    .bind(address_book_entry_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql_error)?
    .ok_or(AddressBookRepositoryError::NotFound)?;
    let entry = entry_from_row(row);
    require_visible_device(pool, owner_user_uuid, entry.device_uuid).await?;
    Ok(entry)
}

pub async fn create_address_book_entry(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    device_uuid: Uuid,
    alias: &str,
    notes: Option<String>,
    position: u32,
) -> Result<AddressBookEntry, AddressBookRepositoryError> {
    let entry = AddressBookEntry::new(
        Uuid::new_v4(),
        address_book_uuid,
        device_uuid,
        alias,
        notes,
        position,
    )?;
    require_visible_device(pool, owner_user_uuid, device_uuid).await?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_owned_book(&mut tx, address_book_uuid, owner_user_uuid).await?;
    sqlx::query(
        "INSERT INTO address_book_entries
         (address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(entry.address_book_entry_uuid.to_string())
    .bind(entry.address_book_uuid.to_string())
    .bind(entry.device_uuid.to_string())
    .bind(&entry.alias)
    .bind(entry.notes.as_deref())
    .bind(i64::from(entry.position))
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)?;
    Ok(entry)
}

pub async fn update_address_book_entry(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_entry_uuid: Uuid,
    device_uuid: Uuid,
    alias: &str,
    notes: Option<String>,
    position: u32,
) -> Result<AddressBookEntry, AddressBookRepositoryError> {
    let current = find_address_book_entry(pool, owner_user_uuid, address_book_entry_uuid).await?;
    require_visible_device(pool, owner_user_uuid, device_uuid).await?;
    let entry = AddressBookEntry::new(
        address_book_entry_uuid,
        current.address_book_uuid,
        device_uuid,
        alias,
        notes,
        position,
    )?;
    let result = sqlx::query(
        "UPDATE address_book_entries SET device_uuid = ?, alias = ?, notes = ?, position = ?
         WHERE address_book_entry_uuid = ?
           AND address_book_uuid IN (
             SELECT address_book_uuid FROM address_books WHERE owner_user_uuid = ?
           )",
    )
    .bind(entry.device_uuid.to_string())
    .bind(&entry.alias)
    .bind(entry.notes.as_deref())
    .bind(i64::from(entry.position))
    .bind(entry.address_book_entry_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(entry)
}

pub async fn delete_address_book_entry(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_entry_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let result = sqlx::query(
        "DELETE FROM address_book_entries
         WHERE address_book_entry_uuid = ?
           AND address_book_uuid IN (
             SELECT address_book_uuid FROM address_books WHERE owner_user_uuid = ?
           )",
    )
    .bind(address_book_entry_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(())
}
