use sqlx::SqlitePool;
use uuid::Uuid;

use super::{map_sql_error, parse_uuid, AddressBookRepositoryError};
use crate::domain::address_book::AddressBookAccessRule;

async fn require_shared_owned_book(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT book_kind FROM address_books
         WHERE address_book_uuid = ? AND owner_user_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .bind(owner_user_uuid.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql_error)?;
    match kind.as_deref() {
        Some("shared") => Ok(()),
        Some(_) => Err(AddressBookRepositoryError::Forbidden),
        None => Err(AddressBookRepositoryError::NotFound),
    }
}

async fn require_principal(
    pool: &SqlitePool,
    principal_type: &str,
    principal_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let sql = match principal_type {
        "user" => "SELECT EXISTS(SELECT 1 FROM users WHERE user_uuid = ?)",
        "group" => "SELECT EXISTS(SELECT 1 FROM access_groups WHERE access_group_uuid = ?)",
        _ => return Err(AddressBookRepositoryError::NotFound),
    };
    let exists: i64 = sqlx::query_scalar(sql)
        .bind(principal_uuid.to_string())
        .fetch_one(pool)
        .await
        .map_err(map_sql_error)?;
    if exists == 0 {
        Err(AddressBookRepositoryError::NotFound)
    } else {
        Ok(())
    }
}

pub async fn list_address_book_access_rules(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<Vec<AddressBookAccessRule>, AddressBookRepositoryError> {
    require_shared_owned_book(pool, owner_user_uuid, address_book_uuid).await?;
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT address_book_uuid, principal_type, principal_uuid, permission
         FROM address_book_access_rules
         WHERE address_book_uuid = ?
         ORDER BY principal_type ASC, principal_uuid ASC",
    )
    .bind(address_book_uuid.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_sql_error)?;
    Ok(rows
        .into_iter()
        .map(
            |(book, principal_type, principal_uuid, permission)| AddressBookAccessRule {
                address_book_uuid: parse_uuid(book),
                principal_type,
                principal_uuid: parse_uuid(principal_uuid),
                permission,
            },
        )
        .collect())
}

pub async fn upsert_address_book_access_rule(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    principal_type: &str,
    principal_uuid: Uuid,
    permission: &str,
) -> Result<AddressBookAccessRule, AddressBookRepositoryError> {
    if !matches!(principal_type, "user" | "group")
        || !matches!(permission, "read" | "write" | "admin")
    {
        return Err(AddressBookRepositoryError::NotFound);
    }
    if principal_type == "user" && principal_uuid == owner_user_uuid {
        return Err(AddressBookRepositoryError::Conflict);
    }
    require_shared_owned_book(pool, owner_user_uuid, address_book_uuid).await?;
    require_principal(pool, principal_type, principal_uuid).await?;
    sqlx::query(
        "INSERT INTO address_book_access_rules
         (address_book_uuid, principal_type, principal_uuid, permission)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(address_book_uuid, principal_type, principal_uuid)
         DO UPDATE SET permission = excluded.permission",
    )
    .bind(address_book_uuid.to_string())
    .bind(principal_type)
    .bind(principal_uuid.to_string())
    .bind(permission)
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    Ok(AddressBookAccessRule {
        address_book_uuid,
        principal_type: principal_type.to_string(),
        principal_uuid,
        permission: permission.to_string(),
    })
}

pub async fn delete_address_book_access_rule(
    pool: &SqlitePool,
    owner_user_uuid: Uuid,
    address_book_uuid: Uuid,
    principal_type: &str,
    principal_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    if !matches!(principal_type, "user" | "group") {
        return Err(AddressBookRepositoryError::NotFound);
    }
    require_shared_owned_book(pool, owner_user_uuid, address_book_uuid).await?;
    let result = sqlx::query(
        "DELETE FROM address_book_access_rules
         WHERE address_book_uuid = ? AND principal_type = ? AND principal_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .bind(principal_type)
    .bind(principal_uuid.to_string())
    .execute(pool)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    Ok(())
}
