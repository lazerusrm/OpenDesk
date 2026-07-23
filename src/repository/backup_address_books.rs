use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::domain::backup::{BackupAddressBook, BackupAddressBookAccessRule};

pub async fn export_address_book_access(
    pool: &SqlitePool,
) -> Result<(Vec<BackupAddressBook>, Vec<BackupAddressBookAccessRule>), sqlx::Error> {
    let books = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT address_book_uuid, owner_user_uuid, name, book_kind
         FROM address_books ORDER BY address_book_uuid ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(
        |(address_book_uuid, owner_user_uuid, name, book_kind)| BackupAddressBook {
            address_book_uuid: Uuid::parse_str(&address_book_uuid).expect("stored uuid"),
            owner_user_uuid: Uuid::parse_str(&owner_user_uuid).expect("stored uuid"),
            name,
            book_kind,
        },
    )
    .collect();
    let rules = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT address_book_uuid, principal_type, principal_uuid, permission
         FROM address_book_access_rules
         ORDER BY address_book_uuid, principal_type, principal_uuid",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(
        |(address_book_uuid, principal_type, principal_uuid, permission)| {
            BackupAddressBookAccessRule {
                address_book_uuid: Uuid::parse_str(&address_book_uuid).expect("stored uuid"),
                principal_type,
                principal_uuid: Uuid::parse_str(&principal_uuid).expect("stored uuid"),
                permission,
            }
        },
    )
    .collect();
    Ok((books, rules))
}

pub async fn restore_address_book_access_rules(
    connection: &mut SqliteConnection,
    rules: &[BackupAddressBookAccessRule],
) -> Result<(), sqlx::Error> {
    for rule in rules {
        sqlx::query(
            "INSERT INTO address_book_access_rules
             (address_book_uuid, principal_type, principal_uuid, permission)
             VALUES (?, ?, ?, ?)",
        )
        .bind(rule.address_book_uuid.to_string())
        .bind(&rule.principal_type)
        .bind(rule.principal_uuid.to_string())
        .bind(&rule.permission)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}
