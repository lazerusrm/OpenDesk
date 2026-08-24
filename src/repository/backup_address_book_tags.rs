use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::domain::backup::{BackupAddressBookEntryTag, BackupAddressBookTag};

pub async fn export_address_book_tags(
    pool: &SqlitePool,
) -> Result<(Vec<BackupAddressBookTag>, Vec<BackupAddressBookEntryTag>), sqlx::Error> {
    let tags = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT address_book_uuid, name, color
         FROM address_book_tags ORDER BY address_book_uuid, name",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(book, name, color)| BackupAddressBookTag {
        address_book_uuid: Uuid::parse_str(&book).expect("stored uuid"),
        name,
        color,
    })
    .collect();
    let entry_tags = sqlx::query_as::<_, (String, String, String)>(
        "SELECT address_book_entry_uuid, address_book_uuid, tag_name
         FROM address_book_entry_tags
         ORDER BY address_book_entry_uuid, tag_name",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(entry, book, tag_name)| BackupAddressBookEntryTag {
        address_book_entry_uuid: Uuid::parse_str(&entry).expect("stored uuid"),
        address_book_uuid: Uuid::parse_str(&book).expect("stored uuid"),
        tag_name,
    })
    .collect();
    Ok((tags, entry_tags))
}

pub async fn restore_address_book_tags(
    connection: &mut SqliteConnection,
    tags: &[BackupAddressBookTag],
    entry_tags: &[BackupAddressBookEntryTag],
) -> Result<(), sqlx::Error> {
    for tag in tags {
        sqlx::query(
            "INSERT INTO address_book_tags (address_book_uuid, name, color) VALUES (?, ?, ?)",
        )
        .bind(tag.address_book_uuid.to_string())
        .bind(&tag.name)
        .bind(tag.color)
        .execute(&mut *connection)
        .await?;
    }
    for link in entry_tags {
        sqlx::query(
            "INSERT INTO address_book_entry_tags
             (address_book_entry_uuid, address_book_uuid, tag_name) VALUES (?, ?, ?)",
        )
        .bind(link.address_book_entry_uuid.to_string())
        .bind(link.address_book_uuid.to_string())
        .bind(&link.tag_name)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}
