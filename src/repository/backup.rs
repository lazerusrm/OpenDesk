use sqlx::SqlitePool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::access_group::AccessGroup;
use crate::domain::access_group_membership::AccessGroupMembership;
use crate::domain::address_book::AddressBookEntry;
use crate::domain::backup::{
    BackupDeviceTag, BackupDocument, BackupEnrollmentToken, BackupSensitivity, BackupUser,
    BACKUP_SCHEMA_VERSION,
};
use crate::domain::device_visibility::{DeviceVisibilityGrant, UserDeviceVisibilityGrant};
use crate::time_format::format_timestamp;

use super::backup_address_book_tags::export_address_book_tags;
use super::backup_address_books::export_address_book_access;
pub use super::backup_restore::restore_backup_document;
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
    #[error("backup restore is blocked while unconsumed migration verifiers remain")]
    UnconsumedMigrationVerifiers,
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
    let (address_books, address_book_access_rules) = export_address_book_access(pool).await?;
    let (address_book_tags, address_book_entry_tags) = export_address_book_tags(pool).await?;
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
            activation_state: user.activation_state,
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
        address_book_access_rules,
        address_book_tags,
        address_book_entries,
        address_book_entry_tags,
        server_config,
        enrollment_tokens,
        users,
    })
}
