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

#[derive(Debug, Error)]
pub enum AddressBookRepositoryError {
    #[error("database error")]
    Database(#[source] sqlx::Error),
    #[error("address-book validation failed: {0}")]
    Validation(#[from] AddressBookValidationError),
    #[error("address book or entry not found")]
    NotFound,
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

pub async fn create_address_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    name: &str,
) -> Result<AddressBook, AddressBookRepositoryError> {
    let book = AddressBook::new(Uuid::new_v4(), owner_user_uuid, name)?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_user(&mut tx, owner_user_uuid).await?;
    sqlx::query(
        "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name) VALUES (?, ?, ?)",
    )
    .bind(book.address_book_uuid.to_string())
    .bind(book.owner_user_uuid.to_string())
    .bind(&book.name)
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)?;
    Ok(book)
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
