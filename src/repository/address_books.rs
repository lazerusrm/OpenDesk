use sqlx::{Sqlite, SqlitePool, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::address_book::{AddressBook, AddressBookValidationError};

#[path = "address_book_entries.rs"]
mod address_book_entries;
pub use address_book_entries::{
    create_address_book_entry, delete_address_book_entry, find_address_book_entry,
    list_address_book_entries, update_address_book_entry,
};

#[path = "address_book_access.rs"]
mod address_book_access;
pub use address_book_access::{
    delete_address_book_access_rule, list_address_book_access_rules,
    upsert_address_book_access_rule,
};

#[path = "personal_address_book.rs"]
mod personal_address_book;
pub use personal_address_book::{
    ensure_personal_address_book, hide_personal_address_book_device,
    sync_visible_devices_into_personal_address_book,
};

#[derive(Debug, Error)]
pub enum AddressBookRepositoryError {
    #[error("database error")]
    Database(#[source] sqlx::Error),
    #[error("address-book validation failed: {0}")]
    Validation(#[from] AddressBookValidationError),
    #[error("address book or entry not found")]
    NotFound,
    #[error("address book access is read-only")]
    Forbidden,
    #[error("address book or entry conflicts with an existing record")]
    Conflict,
}

pub(crate) fn map_sql_error(error: sqlx::Error) -> AddressBookRepositoryError {
    let conflict = error
        .as_database_error()
        .map(|database_error| {
            database_error
                .message()
                .contains("UNIQUE constraint failed")
                || database_error
                    .message()
                    .contains("PRIMARY KEY constraint failed")
        })
        .unwrap_or(false);
    if conflict {
        AddressBookRepositoryError::Conflict
    } else {
        AddressBookRepositoryError::Database(error)
    }
}

pub(crate) fn parse_uuid(value: String) -> Uuid {
    Uuid::parse_str(&value).expect("stored uuid")
}

async fn require_user(
    tx: &mut Transaction<'_, Sqlite>,
    user_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let row = sqlx::query("SELECT 1 FROM users WHERE user_uuid = ? LIMIT 1")
        .bind(user_uuid.to_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sql_error)?;
    if row.is_none() {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(())
}

pub(crate) async fn require_owned_book(
    tx: &mut Transaction<'_, Sqlite>,
    address_book_uuid: Uuid,
    owner_user_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let row = sqlx::query(
        "SELECT 1 FROM address_books
         WHERE address_book_uuid = ? AND owner_user_uuid = ? LIMIT 1",
    )
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sql_error)?;
    if row.is_none() {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(())
}

fn address_book_from_row((book, owner, name): (String, String, String)) -> AddressBook {
    AddressBook {
        address_book_uuid: parse_uuid(book),
        owner_user_uuid: parse_uuid(owner),
        name,
    }
}

pub async fn list_address_books_for_owner(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
) -> Result<Vec<AddressBook>, AddressBookRepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_user(&mut tx, owner_user_uuid).await?;
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT address_book_uuid, owner_user_uuid, name
         FROM address_books WHERE owner_user_uuid = ?
         ORDER BY name ASC, address_book_uuid ASC",
    )
    .bind(owner_user_uuid.to_string())
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)?;
    Ok(rows.into_iter().map(address_book_from_row).collect())
}

pub async fn find_address_book_for_owner(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<AddressBook, AddressBookRepositoryError> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT address_book_uuid, owner_user_uuid, name
         FROM address_books
         WHERE address_book_uuid = ? AND owner_user_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql_error)?
    .ok_or(AddressBookRepositoryError::NotFound)?;
    Ok(address_book_from_row(row))
}

pub async fn find_address_book_for_client_access(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<(AddressBook, String), AddressBookRepositoryError> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT book.address_book_uuid, book.owner_user_uuid, book.name,
                CASE WHEN book.owner_user_uuid = ? THEN 'admin'
                     WHEN MAX(CASE rule.permission
                         WHEN 'read' THEN 1 WHEN 'write' THEN 2 WHEN 'admin' THEN 3 ELSE 0 END) = 3
                         THEN 'admin'
                     WHEN MAX(CASE rule.permission
                         WHEN 'read' THEN 1 WHEN 'write' THEN 2 WHEN 'admin' THEN 3 ELSE 0 END) = 2
                         THEN 'write'
                     ELSE 'read' END
         FROM address_books book
         LEFT JOIN address_book_access_rules rule
           ON rule.address_book_uuid = book.address_book_uuid
          AND (
              (rule.principal_type = 'user' AND rule.principal_uuid = ?)
              OR (rule.principal_type = 'group' AND EXISTS (
                  SELECT 1 FROM access_group_memberships membership
                  WHERE membership.user_uuid = ?
                    AND membership.access_group_uuid = rule.principal_uuid
              ))
          )
         WHERE book.address_book_uuid = ?
           AND (book.owner_user_uuid = ? OR rule.address_book_uuid IS NOT NULL)
         GROUP BY book.address_book_uuid, book.owner_user_uuid, book.name",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(address_book_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql_error)?
    .ok_or(AddressBookRepositoryError::NotFound)?;
    Ok((address_book_from_row((row.0, row.1, row.2)), row.3))
}

pub async fn find_personal_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
) -> Result<AddressBook, AddressBookRepositoryError> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT address_book_uuid, owner_user_uuid, name
         FROM address_books
         WHERE owner_user_uuid = ? AND book_kind = 'personal'",
    )
    .bind(owner_user_uuid.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql_error)?
    .ok_or(AddressBookRepositoryError::NotFound)?;
    Ok(address_book_from_row(row))
}

pub async fn list_shared_address_books_for_user(
    pool: &SqlitePool,
    user_uuid: Uuid,
) -> Result<Vec<(AddressBook, String, String)>, AddressBookRepositoryError> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT book.address_book_uuid, book.owner_user_uuid, book.name,
                owner.username,
                CASE WHEN book.owner_user_uuid = ? THEN 'admin'
                     ELSE CASE MAX(CASE rule.permission
                         WHEN 'read' THEN 1 WHEN 'write' THEN 2 WHEN 'admin' THEN 3 ELSE 0 END)
                         WHEN 3 THEN 'admin' WHEN 2 THEN 'write' ELSE 'read' END
                END
         FROM address_books book
         JOIN users owner ON owner.user_uuid = book.owner_user_uuid
         LEFT JOIN address_book_access_rules rule ON rule.address_book_uuid = book.address_book_uuid
         WHERE book.book_kind = 'shared'
           AND (
               book.owner_user_uuid = ?
               OR (rule.principal_type = 'user' AND rule.principal_uuid = ?)
               OR (rule.principal_type = 'group' AND EXISTS (
                   SELECT 1 FROM access_group_memberships membership
                   WHERE membership.user_uuid = ?
                     AND membership.access_group_uuid = rule.principal_uuid
               ))
           )
         GROUP BY book.address_book_uuid, book.owner_user_uuid, book.name, owner.username
         ORDER BY book.name ASC, book.address_book_uuid ASC",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql_error)?;
    Ok(rows
        .into_iter()
        .map(|(book, owner, name, owner_name, permission)| {
            (
                address_book_from_row((book, owner, name)),
                owner_name,
                permission,
            )
        })
        .collect())
}

pub async fn list_address_book_tags_for_client(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<Vec<(String, i64)>, AddressBookRepositoryError> {
    find_address_book_for_client_access(pool, user_uuid, address_book_uuid).await?;
    sqlx::query_as(
        "SELECT name, color FROM address_book_tags
         WHERE address_book_uuid = ? ORDER BY name ASC",
    )
    .bind(address_book_uuid.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql_error)
}

async fn create_address_book_with_kind(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    name: &str,
    book_kind: &str,
) -> Result<AddressBook, AddressBookRepositoryError> {
    let book = AddressBook::new(Uuid::new_v4(), owner_user_uuid, name)?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_user(&mut tx, owner_user_uuid).await?;
    if book_kind == "personal" {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM address_books
             WHERE owner_user_uuid = ? AND book_kind = 'personal')",
        )
        .bind(owner_user_uuid.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sql_error)?;
        if exists {
            return Err(AddressBookRepositoryError::Conflict);
        }
    }
    sqlx::query(
        "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name, book_kind)
         VALUES (?, ?, ?, ?)",
    )
    .bind(book.address_book_uuid.to_string())
    .bind(book.owner_user_uuid.to_string())
    .bind(&book.name)
    .bind(book_kind)
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)?;
    Ok(book)
}

pub async fn create_personal_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    name: &str,
) -> Result<AddressBook, AddressBookRepositoryError> {
    create_address_book_with_kind(pool, owner_user_uuid, name, "personal").await
}

pub async fn create_shared_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    name: &str,
) -> Result<AddressBook, AddressBookRepositoryError> {
    create_address_book_with_kind(pool, owner_user_uuid, name, "shared").await
}

pub async fn rename_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    name: &str,
) -> Result<AddressBook, AddressBookRepositoryError> {
    let book = AddressBook::new(address_book_uuid, owner_user_uuid, name)?;
    let result = sqlx::query(
        "UPDATE address_books SET name = ?
         WHERE address_book_uuid = ? AND owner_user_uuid = ?",
    )
    .bind(&book.name)
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(book)
}

pub async fn delete_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let result = sqlx::query(
        "DELETE FROM address_books
         WHERE address_book_uuid = ? AND owner_user_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(())
}
