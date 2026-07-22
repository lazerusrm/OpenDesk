use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::access_group::AccessGroup;
use crate::domain::access_group_membership::AccessGroupMembership;
use crate::domain::address_book::{AddressBook, AddressBookEntry};
use crate::domain::backup::{
    BackupDeviceTag, BackupDocument, BackupEnrollmentToken, BackupSensitivity, BackupUser,
    BACKUP_SCHEMA_VERSION,
};
use crate::domain::device_visibility::{DeviceVisibilityGrant, UserDeviceVisibilityGrant};
use crate::time_format::format_timestamp;

use super::devices::list_devices;
use super::enrollment_tokens::list_enrollment_tokens;
use super::server_config::load_server_config;
use super::sites::list_sites;
use super::tags::list_device_tag_links;
use super::tags::list_tags;
use super::users::list_users;

#[derive(Debug, thiserror::Error)]
pub enum BackupRestoreError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("backup validation failed: {0}")]
    Validation(#[from] crate::domain::backup::BackupValidationError),
}

pub async fn export_backup_document(pool: &SqlitePool) -> Result<BackupDocument, sqlx::Error> {
    let sites = list_sites(pool).await?;
    let tags = list_tags(pool).await?;
    let devices = list_devices(pool).await?;
    let device_tags = list_device_tag_links(pool)
        .await?
        .into_iter()
        .map(|(device_uuid, tag_uuid)| BackupDeviceTag {
            device_uuid,
            tag_uuid,
        })
        .collect();
    let access_groups = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, name FROM access_groups ORDER BY access_group_uuid ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(access_group_uuid, name)| AccessGroup {
        access_group_uuid: Uuid::parse_str(&access_group_uuid).expect("stored uuid"),
        name,
    })
    .collect();
    let access_group_memberships = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, user_uuid
         FROM access_group_memberships ORDER BY access_group_uuid ASC, user_uuid ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(access_group_uuid, user_uuid)| AccessGroupMembership {
        access_group_uuid: Uuid::parse_str(&access_group_uuid).expect("stored uuid"),
        user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
    })
    .collect();
    let device_visibility_grants = sqlx::query_as::<_, (String, String)>(
        "SELECT access_group_uuid, device_uuid
         FROM device_visibility_grants ORDER BY access_group_uuid ASC, device_uuid ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(access_group_uuid, device_uuid)| DeviceVisibilityGrant {
        access_group_uuid: Uuid::parse_str(&access_group_uuid).expect("stored uuid"),
        device_uuid: Uuid::parse_str(&device_uuid).expect("stored uuid"),
    })
    .collect();
    let user_device_visibility_grants = sqlx::query_as::<_, (String, String)>(
        "SELECT user_uuid, device_uuid
         FROM user_device_visibility_grants ORDER BY user_uuid ASC, device_uuid ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(user_uuid, device_uuid)| UserDeviceVisibilityGrant {
        user_uuid: Uuid::parse_str(&user_uuid).expect("stored uuid"),
        device_uuid: Uuid::parse_str(&device_uuid).expect("stored uuid"),
    })
    .collect();
    let address_books = sqlx::query_as::<_, (String, String, String)>(
        "SELECT address_book_uuid, owner_user_uuid, name
         FROM address_books ORDER BY address_book_uuid ASC",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(address_book_uuid, owner_user_uuid, name)| AddressBook {
        address_book_uuid: Uuid::parse_str(&address_book_uuid).expect("stored uuid"),
        owner_user_uuid: Uuid::parse_str(&owner_user_uuid).expect("stored uuid"),
        name,
    })
    .collect();
    let address_book_entries =
        sqlx::query_as::<_, (String, String, String, String, Option<String>, i64)>(
            "SELECT address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position
         FROM address_book_entries ORDER BY address_book_uuid ASC, position ASC",
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(
            |(address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position)| {
                AddressBookEntry {
                    address_book_entry_uuid: Uuid::parse_str(&address_book_entry_uuid)
                        .expect("stored uuid"),
                    address_book_uuid: Uuid::parse_str(&address_book_uuid).expect("stored uuid"),
                    device_uuid: Uuid::parse_str(&device_uuid).expect("stored uuid"),
                    alias,
                    notes,
                    position: u32::try_from(position).expect("stored nonnegative position"),
                }
            },
        )
        .collect();
    let server_config = load_server_config(pool).await?;
    let enrollment_tokens = list_enrollment_tokens(pool)
        .await?
        .into_iter()
        .map(|token| BackupEnrollmentToken {
            enrollment_token_uuid: token.enrollment_token_uuid,
            token_hash: token.token_hash,
            label: token.label,
            site_uuid: token.site_uuid,
            expires_at: token.expires_at.map(format_timestamp),
            revoked_at: token.revoked_at.map(format_timestamp),
        })
        .collect();
    let users = list_users(pool)
        .await?
        .into_iter()
        .map(|user| BackupUser {
            user_uuid: user.user_uuid,
            username: user.username,
            password_hash: user.password_hash,
            role: user.role,
        })
        .collect();
    Ok(BackupDocument {
        schema_version: BACKUP_SCHEMA_VERSION,
        exported_at: format_timestamp(OffsetDateTime::now_utc()),
        sensitivity: BackupSensitivity::default(),
        sites,
        tags,
        devices,
        device_tags,
        access_groups,
        access_group_memberships,
        device_visibility_grants,
        user_device_visibility_grants,
        address_books,
        address_book_entries,
        server_config,
        enrollment_tokens,
        users,
    })
}

pub async fn restore_backup_document(
    pool: &SqlitePool,
    document: &BackupDocument,
) -> Result<(), BackupRestoreError> {
    crate::domain::backup::validate_backup_document(document)?;
    let now = format_timestamp(OffsetDateTime::now_utc());
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM device_tags")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM address_book_entries")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM address_books")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM user_device_visibility_grants")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM device_visibility_grants")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM access_group_memberships")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM access_groups")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM endpoint_checkins")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM devices").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM enrollment_tokens")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tags").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM sites").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM server_configs")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM users").execute(&mut *tx).await?;

    for site in &document.sites {
        sqlx::query(
            "INSERT INTO sites (site_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(site.site_uuid.to_string())
        .bind(&site.name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for tag in &document.tags {
        sqlx::query(
            "INSERT INTO tags (tag_uuid, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind(tag.tag_uuid.to_string())
        .bind(&tag.name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for device in &document.devices {
        sqlx::query(
            "INSERT INTO devices (
                device_uuid, rustdesk_id, alias, hostname, os_family, os_version, architecture,
                rustdesk_version, site_uuid, owner, notes, last_checkin_at, archived,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(device.device_uuid.to_string())
        .bind(device.rustdesk_id.as_deref())
        .bind(&device.alias)
        .bind(device.hostname.as_deref())
        .bind(device.os_family.as_deref())
        .bind(device.os_version.as_deref())
        .bind(device.architecture.as_deref())
        .bind(device.rustdesk_version.as_deref())
        .bind(device.site_uuid.map(|value| value.to_string()))
        .bind(device.owner.as_deref())
        .bind(device.notes.as_deref())
        .bind(device.last_checkin_at.as_deref())
        .bind(if device.archived { 1 } else { 0 })
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for link in &document.device_tags {
        sqlx::query("INSERT INTO device_tags (device_uuid, tag_uuid) VALUES (?, ?)")
            .bind(link.device_uuid.to_string())
            .bind(link.tag_uuid.to_string())
            .execute(&mut *tx)
            .await?;
    }
    for user in &document.users {
        sqlx::query(
            "INSERT INTO users (user_uuid, username, password_hash, role, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user.user_uuid.to_string())
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for group in &document.access_groups {
        sqlx::query("INSERT INTO access_groups (access_group_uuid, name) VALUES (?, ?)")
            .bind(group.access_group_uuid.to_string())
            .bind(&group.name)
            .execute(&mut *tx)
            .await?;
    }
    for book in &document.address_books {
        sqlx::query(
            "INSERT INTO address_books (address_book_uuid, owner_user_uuid, name)
             VALUES (?, ?, ?)",
        )
        .bind(book.address_book_uuid.to_string())
        .bind(book.owner_user_uuid.to_string())
        .bind(&book.name)
        .execute(&mut *tx)
        .await?;
    }
    for membership in &document.access_group_memberships {
        sqlx::query(
            "INSERT INTO access_group_memberships (access_group_uuid, user_uuid)
             VALUES (?, ?)",
        )
        .bind(membership.access_group_uuid.to_string())
        .bind(membership.user_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    for grant in &document.device_visibility_grants {
        sqlx::query(
            "INSERT INTO device_visibility_grants (access_group_uuid, device_uuid)
             VALUES (?, ?)",
        )
        .bind(grant.access_group_uuid.to_string())
        .bind(grant.device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    for grant in &document.user_device_visibility_grants {
        sqlx::query(
            "INSERT INTO user_device_visibility_grants (user_uuid, device_uuid)
             VALUES (?, ?)",
        )
        .bind(grant.user_uuid.to_string())
        .bind(grant.device_uuid.to_string())
        .execute(&mut *tx)
        .await?;
    }
    for entry in &document.address_book_entries {
        sqlx::query(
            "INSERT INTO address_book_entries (
                address_book_entry_uuid, address_book_uuid, device_uuid, alias, notes, position
             ) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry.address_book_entry_uuid.to_string())
        .bind(entry.address_book_uuid.to_string())
        .bind(entry.device_uuid.to_string())
        .bind(&entry.alias)
        .bind(entry.notes.as_deref())
        .bind(i64::from(entry.position))
        .execute(&mut *tx)
        .await?;
    }
    if let Some(config) = &document.server_config {
        sqlx::query(
            "INSERT INTO server_configs (
                server_config_uuid, id_server, relay_server, api_server, public_key,
                updated_at, updated_by_user_uuid
             ) VALUES (?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&config.id_server)
        .bind(&config.relay_server)
        .bind(&config.api_server)
        .bind(&config.public_key)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    for token in &document.enrollment_tokens {
        sqlx::query(
            "INSERT INTO enrollment_tokens (
                enrollment_token_uuid, token_hash, label, site_uuid, expires_at, revoked_at, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(token.enrollment_token_uuid.to_string())
        .bind(&token.token_hash)
        .bind(&token.label)
        .bind(token.site_uuid.map(|value| value.to_string()))
        .bind(token.expires_at.as_deref())
        .bind(token.revoked_at.as_deref())
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
