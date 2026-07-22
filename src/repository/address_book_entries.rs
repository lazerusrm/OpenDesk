use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use super::{map_sql_error, parse_uuid, require_owned_book, AddressBookRepositoryError};
use crate::domain::address_book::AddressBookEntry;

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

/// Device visibility is an explicit direct grant or an explicit shared group grant.
/// Roles, site membership, tags, and device owner are deliberately not consulted.
async fn require_visible_device(
    tx: &mut Transaction<'_, Sqlite>,
    owner_user_uuid: Uuid,
    device_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let row = sqlx::query(
        "SELECT 1
         FROM devices device
         WHERE device.device_uuid = ?
           AND (
             EXISTS (
               SELECT 1 FROM user_device_visibility_grants direct_grant
               WHERE direct_grant.user_uuid = ?
                 AND direct_grant.device_uuid = device.device_uuid
             )
             OR EXISTS (
               SELECT 1
               FROM access_group_memberships membership
               INNER JOIN device_visibility_grants group_grant
                 ON group_grant.access_group_uuid = membership.access_group_uuid
               WHERE membership.user_uuid = ?
                 AND group_grant.device_uuid = device.device_uuid
             )
           )
         LIMIT 1",
    )
    .bind(device_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sql_error)?;
    if row.is_none() {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(())
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
         INNER JOIN address_books book ON book.address_book_uuid = entry.address_book_uuid
         WHERE entry.address_book_uuid = ? AND book.owner_user_uuid = ?
           AND (
             EXISTS (
               SELECT 1 FROM user_device_visibility_grants direct_grant
               WHERE direct_grant.user_uuid = ? AND direct_grant.device_uuid = entry.device_uuid
             )
             OR EXISTS (
               SELECT 1
               FROM access_group_memberships membership
               INNER JOIN device_visibility_grants group_grant
                 ON group_grant.access_group_uuid = membership.access_group_uuid
               WHERE membership.user_uuid = ? AND group_grant.device_uuid = entry.device_uuid
             )
           )
         ORDER BY entry.position ASC, entry.address_book_entry_uuid ASC",
    )
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)?;
    Ok(rows.into_iter().map(entry_from_row).collect())
}

pub async fn find_address_book_entry(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_entry_uuid: Uuid,
) -> Result<AddressBookEntry, AddressBookRepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, i64)>(
        "SELECT entry.address_book_entry_uuid, entry.address_book_uuid, entry.device_uuid,
                entry.alias, entry.notes, entry.position
         FROM address_book_entries entry
         INNER JOIN address_books book ON book.address_book_uuid = entry.address_book_uuid
         WHERE entry.address_book_entry_uuid = ? AND book.owner_user_uuid = ?
           AND (
             EXISTS (
               SELECT 1 FROM user_device_visibility_grants direct_grant
               WHERE direct_grant.user_uuid = ? AND direct_grant.device_uuid = entry.device_uuid
             )
             OR EXISTS (
               SELECT 1
               FROM access_group_memberships membership
               INNER JOIN device_visibility_grants group_grant
                 ON group_grant.access_group_uuid = membership.access_group_uuid
               WHERE membership.user_uuid = ? AND group_grant.device_uuid = entry.device_uuid
             )
           )",
    )
    .bind(address_book_entry_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sql_error)?
    .ok_or(AddressBookRepositoryError::NotFound)?;
    tx.commit().await.map_err(map_sql_error)?;
    Ok(entry_from_row(row))
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
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_owned_book(&mut tx, address_book_uuid, owner_user_uuid).await?;
    require_visible_device(&mut tx, owner_user_uuid, device_uuid).await?;
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
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, i64)>(
        "SELECT entry.address_book_entry_uuid, entry.address_book_uuid, entry.device_uuid,
                entry.alias, entry.notes, entry.position
         FROM address_book_entries entry
         INNER JOIN address_books book ON book.address_book_uuid = entry.address_book_uuid
         WHERE entry.address_book_entry_uuid = ? AND book.owner_user_uuid = ?
           AND (
             EXISTS (
               SELECT 1 FROM user_device_visibility_grants direct_grant
               WHERE direct_grant.user_uuid = ? AND direct_grant.device_uuid = entry.device_uuid
             )
             OR EXISTS (
               SELECT 1
               FROM access_group_memberships membership
               INNER JOIN device_visibility_grants group_grant
                 ON group_grant.access_group_uuid = membership.access_group_uuid
               WHERE membership.user_uuid = ? AND group_grant.device_uuid = entry.device_uuid
             )
           )",
    )
    .bind(address_book_entry_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sql_error)?
    .ok_or(AddressBookRepositoryError::NotFound)?;
    let current = entry_from_row(row);
    let entry = AddressBookEntry::new(
        address_book_entry_uuid,
        current.address_book_uuid,
        device_uuid,
        alias,
        notes,
        position,
    )?;
    require_owned_book(&mut tx, entry.address_book_uuid, owner_user_uuid).await?;
    require_visible_device(&mut tx, owner_user_uuid, device_uuid).await?;
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
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    tx.commit().await.map_err(map_sql_error)?;
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
