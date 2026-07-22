use std::collections::HashSet;

use uuid::Uuid;

use super::{BackupDocument, BackupValidationError};
use crate::domain::access_group::validate_access_group_name;
use crate::domain::address_book::{validate_address_book_entry_alias, validate_address_book_name};
use crate::domain::role::Role;
use crate::domain::server_config::validate_server_config;

pub fn validate_backup_document(document: &BackupDocument) -> Result<(), BackupValidationError> {
    if document.schema_version != super::BACKUP_SCHEMA_VERSION {
        return Err(BackupValidationError::UnsupportedSchemaVersion);
    }
    if document.users.is_empty() {
        return Err(BackupValidationError::EmptyUsers);
    }

    let site_ids = unique_ids(document.sites.iter().map(|site| site.site_uuid), "site")?;
    let tag_ids = unique_ids(document.tags.iter().map(|tag| tag.tag_uuid), "tag")?;
    let device_ids = unique_ids(
        document.devices.iter().map(|device| device.device_uuid),
        "device",
    )?;
    let user_ids = unique_ids(document.users.iter().map(|user| user.user_uuid), "user")?;
    let access_group_ids = unique_ids(
        document
            .access_groups
            .iter()
            .map(|group| group.access_group_uuid),
        "access group",
    )?;
    let address_book_ids = unique_ids(
        document
            .address_books
            .iter()
            .map(|book| book.address_book_uuid),
        "address book",
    )?;
    unique_ids(
        document
            .address_book_entries
            .iter()
            .map(|entry| entry.address_book_entry_uuid),
        "address book entry",
    )?;
    unique_ids(
        document
            .enrollment_tokens
            .iter()
            .map(|token| token.enrollment_token_uuid),
        "enrollment token",
    )?;

    let mut usernames = HashSet::new();
    for user in &document.users {
        if !matches!(
            user.role.as_str(),
            Role::ADMIN | Role::OPERATOR | Role::READ_ONLY
        ) {
            return Err(BackupValidationError::InvalidRole);
        }
        if !usernames.insert(&user.username) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "username",
            });
        }
    }

    let mut access_group_names = HashSet::new();
    for group in &document.access_groups {
        if group.name != group.name.trim() {
            return Err(BackupValidationError::NonCanonicalValue {
                field: "access group name",
            });
        }
        validate_access_group_name(&group.name).map_err(|_| {
            BackupValidationError::InvalidValue {
                field: "access group name",
            }
        })?;
        if !access_group_names.insert(&group.name) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "access group name",
            });
        }
    }

    for device in &document.devices {
        if let Some(site_uuid) = device.site_uuid {
            if !site_ids.contains(&site_uuid) {
                return Err(BackupValidationError::InvalidReference {
                    relation: "device site",
                });
            }
        }
    }
    for token in &document.enrollment_tokens {
        if let Some(site_uuid) = token.site_uuid {
            if !site_ids.contains(&site_uuid) {
                return Err(BackupValidationError::InvalidReference {
                    relation: "enrollment token site",
                });
            }
        }
    }
    let mut links = HashSet::new();
    for link in &document.device_tags {
        if !device_ids.contains(&link.device_uuid) || !tag_ids.contains(&link.tag_uuid) {
            return Err(BackupValidationError::InvalidReference {
                relation: "device tag",
            });
        }
        if !links.insert((link.device_uuid, link.tag_uuid)) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "device tag",
            });
        }
    }

    let mut memberships = HashSet::new();
    for membership in &document.access_group_memberships {
        if !access_group_ids.contains(&membership.access_group_uuid)
            || !user_ids.contains(&membership.user_uuid)
        {
            return Err(BackupValidationError::InvalidReference {
                relation: "access group membership",
            });
        }
        if !memberships.insert((membership.access_group_uuid, membership.user_uuid)) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "access group membership",
            });
        }
    }

    let mut group_grants = HashSet::new();
    for grant in &document.device_visibility_grants {
        if !access_group_ids.contains(&grant.access_group_uuid)
            || !device_ids.contains(&grant.device_uuid)
        {
            return Err(BackupValidationError::InvalidReference {
                relation: "device visibility grant",
            });
        }
        if !group_grants.insert((grant.access_group_uuid, grant.device_uuid)) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "device visibility grant",
            });
        }
    }

    let mut user_grants = HashSet::new();
    for grant in &document.user_device_visibility_grants {
        if !user_ids.contains(&grant.user_uuid) || !device_ids.contains(&grant.device_uuid) {
            return Err(BackupValidationError::InvalidReference {
                relation: "user device visibility grant",
            });
        }
        if !user_grants.insert((grant.user_uuid, grant.device_uuid)) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "user device visibility grant",
            });
        }
    }

    let mut address_book_names = HashSet::new();
    for book in &document.address_books {
        if !user_ids.contains(&book.owner_user_uuid) {
            return Err(BackupValidationError::InvalidReference {
                relation: "address book owner",
            });
        }
        if book.name != book.name.trim() {
            return Err(BackupValidationError::NonCanonicalValue {
                field: "address book name",
            });
        }
        validate_address_book_name(&book.name).map_err(|_| {
            BackupValidationError::InvalidValue {
                field: "address book name",
            }
        })?;
        if !address_book_names.insert((book.owner_user_uuid, &book.name)) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "address book owner/name",
            });
        }
    }

    let mut address_book_entries = HashSet::new();
    for entry in &document.address_book_entries {
        if !address_book_ids.contains(&entry.address_book_uuid)
            || !device_ids.contains(&entry.device_uuid)
        {
            return Err(BackupValidationError::InvalidReference {
                relation: "address book entry",
            });
        }
        if entry.alias != entry.alias.trim() {
            return Err(BackupValidationError::NonCanonicalValue {
                field: "address book entry alias",
            });
        }
        validate_address_book_entry_alias(&entry.alias).map_err(|_| {
            BackupValidationError::InvalidValue {
                field: "address book entry alias",
            }
        })?;
        if !address_book_entries.insert((entry.address_book_uuid, entry.device_uuid)) {
            return Err(BackupValidationError::DuplicateIdentifier {
                collection: "address book entry book/device",
            });
        }
    }
    if let Some(config) = &document.server_config {
        if validate_server_config(config).is_err() {
            return Err(BackupValidationError::InvalidServerConfig);
        }
    }
    Ok(())
}

fn unique_ids(
    values: impl Iterator<Item = Uuid>,
    collection: &'static str,
) -> Result<HashSet<Uuid>, BackupValidationError> {
    let mut ids = HashSet::new();
    for id in values {
        if !ids.insert(id) {
            return Err(BackupValidationError::DuplicateIdentifier { collection });
        }
    }
    Ok(ids)
}
