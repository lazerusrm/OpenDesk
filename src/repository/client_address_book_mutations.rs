use sqlx::{Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::domain::address_book::{
    normalize_optional_notes, validate_address_book_entry_alias, AddressBookValidationError,
};

use super::address_books::{map_sql_error, AddressBookRepositoryError};

#[derive(Debug)]
pub struct ClientAddressBookPeerDraft {
    pub rustdesk_id: String,
    pub alias: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

pub struct ClientAddressBookPeerUpdate {
    pub rustdesk_id: String,
    pub alias: Option<String>,
    pub notes: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

async fn require_write_access(
    tx: &mut Transaction<'_, Sqlite>,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
) -> Result<(), AddressBookRepositoryError> {
    let allowed: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM address_books book
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
              AND (book.owner_user_uuid = ? OR rule.permission IN ('write', 'admin'))
        )",
    )
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .bind(address_book_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sql_error)?;
    if !allowed {
        return Err(AddressBookRepositoryError::Forbidden);
    }
    Ok(())
}

async fn visible_device_uuid(
    tx: &mut Transaction<'_, Sqlite>,
    user_uuid: Uuid,
    rustdesk_id: &str,
) -> Result<Uuid, AddressBookRepositoryError> {
    let values: Vec<String> = sqlx::query_scalar(
        "SELECT device.device_uuid FROM devices device
         WHERE device.rustdesk_id = ? AND device.archived = 0
           AND (
               EXISTS (SELECT 1 FROM user_device_visibility_grants grant
                       WHERE grant.user_uuid = ? AND grant.device_uuid = device.device_uuid)
               OR EXISTS (
                   SELECT 1 FROM access_group_memberships membership
                   JOIN device_visibility_grants grant
                     ON grant.access_group_uuid = membership.access_group_uuid
                   WHERE membership.user_uuid = ? AND grant.device_uuid = device.device_uuid
               )
           )",
    )
    .bind(rustdesk_id)
    .bind(user_uuid.to_string())
    .bind(user_uuid.to_string())
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sql_error)?;
    match values.as_slice() {
        [value] => Ok(Uuid::parse_str(value).expect("stored uuid")),
        [] => Err(AddressBookRepositoryError::NotFound),
        _ => Err(AddressBookRepositoryError::Conflict),
    }
}

async fn replace_entry_tags(
    tx: &mut Transaction<'_, Sqlite>,
    address_book_entry_uuid: Uuid,
    address_book_uuid: Uuid,
    tags: &[String],
) -> Result<(), AddressBookRepositoryError> {
    sqlx::query("DELETE FROM address_book_entry_tags WHERE address_book_entry_uuid = ?")
        .bind(address_book_entry_uuid.to_string())
        .execute(&mut **tx)
        .await
        .map_err(map_sql_error)?;
    for tag in tags {
        let result = sqlx::query(
            "INSERT INTO address_book_entry_tags
             (address_book_entry_uuid, address_book_uuid, tag_name)
             SELECT ?, address_book_uuid, name FROM address_book_tags
             WHERE address_book_uuid = ? AND name = ?",
        )
        .bind(address_book_entry_uuid.to_string())
        .bind(address_book_uuid.to_string())
        .bind(tag)
        .execute(&mut **tx)
        .await
        .map_err(map_sql_error)?;
        if result.rows_affected() != 1 {
            return Err(AddressBookRepositoryError::NotFound);
        }
    }
    Ok(())
}

pub async fn add_client_address_book_peer(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    draft: ClientAddressBookPeerDraft,
) -> Result<(), AddressBookRepositoryError> {
    validate_address_book_entry_alias(&draft.alias)?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    let device_uuid = visible_device_uuid(&mut tx, user_uuid, &draft.rustdesk_id).await?;
    let entry_uuid = Uuid::new_v4();
    let position: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM address_book_entries
         WHERE address_book_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .fetch_one(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    sqlx::query(
        "INSERT INTO address_book_entries
         (address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(entry_uuid.to_string())
    .bind(address_book_uuid.to_string())
    .bind(device_uuid.to_string())
    .bind(draft.alias.trim())
    .bind(normalize_optional_notes(draft.notes))
    .bind(position)
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    replace_entry_tags(&mut tx, entry_uuid, address_book_uuid, &draft.tags).await?;
    tx.commit().await.map_err(map_sql_error)
}

pub async fn update_client_address_book_peer(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    update: ClientAddressBookPeerUpdate,
) -> Result<(), AddressBookRepositoryError> {
    if let Some(alias) = &update.alias {
        validate_address_book_entry_alias(alias)?;
    }
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    let device_uuid = visible_device_uuid(&mut tx, user_uuid, &update.rustdesk_id).await?;
    let entry_uuid: Option<String> = sqlx::query_scalar(
        "SELECT address_book_entry_uuid FROM address_book_entries
         WHERE address_book_uuid = ? AND device_uuid = ?",
    )
    .bind(address_book_uuid.to_string())
    .bind(device_uuid.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    let entry_uuid = entry_uuid
        .map(|value| Uuid::parse_str(&value).expect("stored uuid"))
        .ok_or(AddressBookRepositoryError::NotFound)?;
    if let Some(alias) = update.alias {
        sqlx::query("UPDATE address_book_entries SET alias = ? WHERE address_book_entry_uuid = ?")
            .bind(alias.trim())
            .bind(entry_uuid.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_sql_error)?;
    }
    if let Some(notes) = update.notes {
        sqlx::query("UPDATE address_book_entries SET notes = ? WHERE address_book_entry_uuid = ?")
            .bind(normalize_optional_notes(notes))
            .bind(entry_uuid.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_sql_error)?;
    }
    if let Some(tags) = update.tags {
        replace_entry_tags(&mut tx, entry_uuid, address_book_uuid, &tags).await?;
    }
    tx.commit().await.map_err(map_sql_error)
}

pub async fn delete_client_address_book_peers(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    rustdesk_ids: &[String],
) -> Result<(), AddressBookRepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    for rustdesk_id in rustdesk_ids {
        let device_uuid = visible_device_uuid(&mut tx, user_uuid, rustdesk_id).await?;
        let result = sqlx::query(
            "DELETE FROM address_book_entries WHERE address_book_uuid = ? AND device_uuid = ?",
        )
        .bind(address_book_uuid.to_string())
        .bind(device_uuid.to_string())
        .execute(&mut *tx)
        .await
        .map_err(map_sql_error)?;
        if result.rows_affected() != 1 {
            return Err(AddressBookRepositoryError::NotFound);
        }
    }
    tx.commit().await.map_err(map_sql_error)
}

pub async fn add_client_address_book_tag(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    name: &str,
    color: i64,
) -> Result<(), AddressBookRepositoryError> {
    let name = validate_tag_name(name)?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    sqlx::query("INSERT INTO address_book_tags (address_book_uuid, name, color) VALUES (?, ?, ?)")
        .bind(address_book_uuid.to_string())
        .bind(name)
        .bind(color)
        .execute(&mut *tx)
        .await
        .map_err(map_sql_error)?;
    tx.commit().await.map_err(map_sql_error)
}

pub async fn rename_client_address_book_tag(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    old_name: &str,
    new_name: &str,
) -> Result<(), AddressBookRepositoryError> {
    let old_name = validate_tag_name(old_name)?;
    let new_name = validate_tag_name(new_name)?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    let inserted = sqlx::query(
        "INSERT INTO address_book_tags (address_book_uuid, name, color)
         SELECT address_book_uuid, ?, color FROM address_book_tags
         WHERE address_book_uuid = ? AND name = ?",
    )
    .bind(new_name)
    .bind(address_book_uuid.to_string())
    .bind(old_name)
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    if inserted.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    sqlx::query(
        "UPDATE address_book_entry_tags SET tag_name = ?
         WHERE address_book_uuid = ? AND tag_name = ?",
    )
    .bind(new_name)
    .bind(address_book_uuid.to_string())
    .bind(old_name)
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    let deleted =
        sqlx::query("DELETE FROM address_book_tags WHERE address_book_uuid = ? AND name = ?")
            .bind(address_book_uuid.to_string())
            .bind(old_name)
            .execute(&mut *tx)
            .await
            .map_err(map_sql_error)?;
    if deleted.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    tx.commit().await.map_err(map_sql_error)
}

pub async fn update_client_address_book_tag_color(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    name: &str,
    color: i64,
) -> Result<(), AddressBookRepositoryError> {
    let name = validate_tag_name(name)?;
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    let result = sqlx::query(
        "UPDATE address_book_tags SET color = ? WHERE address_book_uuid = ? AND name = ?",
    )
    .bind(color)
    .bind(address_book_uuid.to_string())
    .bind(name)
    .execute(&mut *tx)
    .await
    .map_err(map_sql_error)?;
    if result.rows_affected() != 1 {
        return Err(AddressBookRepositoryError::NotFound);
    }
    tx.commit().await.map_err(map_sql_error)
}

pub async fn delete_client_address_book_tags(
    pool: &SqlitePool,
    user_uuid: Uuid,
    address_book_uuid: Uuid,
    names: &[String],
) -> Result<(), AddressBookRepositoryError> {
    let mut tx = pool.begin().await.map_err(map_sql_error)?;
    require_write_access(&mut tx, user_uuid, address_book_uuid).await?;
    for name in names {
        let name = validate_tag_name(name)?;
        let result =
            sqlx::query("DELETE FROM address_book_tags WHERE address_book_uuid = ? AND name = ?")
                .bind(address_book_uuid.to_string())
                .bind(name)
                .execute(&mut *tx)
                .await
                .map_err(map_sql_error)?;
        if result.rows_affected() != 1 {
            return Err(AddressBookRepositoryError::NotFound);
        }
    }
    tx.commit().await.map_err(map_sql_error)
}

fn validate_tag_name(name: &str) -> Result<&str, AddressBookRepositoryError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(AddressBookRepositoryError::Validation(
            AddressBookValidationError::EmptyAlias,
        ));
    }
    Ok(name)
}
