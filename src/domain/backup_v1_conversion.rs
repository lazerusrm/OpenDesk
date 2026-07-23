use serde::{Deserialize, Serialize};

use super::{
    validate_backup_document, BackupDeviceTag, BackupDocument, BackupEnrollmentToken,
    BackupSensitivity, BackupUser, BACKUP_SCHEMA_VERSION,
};
use crate::domain::{
    access_group::AccessGroup,
    access_group_membership::AccessGroupMembership,
    address_book::{AddressBook, AddressBookEntry},
    device::Device,
    device_visibility::{DeviceVisibilityGrant, UserDeviceVisibilityGrant},
    server_config::ServerConfig,
    site::Site,
    tag::Tag,
};

use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LegacyBackupUser {
    pub user_uuid: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

impl LegacyBackupUser {
    fn into_current(self) -> BackupUser {
        BackupUser {
            user_uuid: self.user_uuid,
            username: self.username,
            password_hash: self.password_hash,
            role: self.role,
            activation_state: "active".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BackupDocumentV1 {
    pub schema_version: u32,
    pub exported_at: String,
    pub sensitivity: BackupSensitivity,
    pub sites: Vec<Site>,
    pub tags: Vec<Tag>,
    pub devices: Vec<Device>,
    pub device_tags: Vec<BackupDeviceTag>,
    pub server_config: Option<ServerConfig>,
    pub enrollment_tokens: Vec<BackupEnrollmentToken>,
    pub users: Vec<LegacyBackupUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BackupDocumentV2 {
    pub schema_version: u32,
    pub exported_at: String,
    pub sensitivity: BackupSensitivity,
    pub sites: Vec<Site>,
    pub tags: Vec<Tag>,
    pub devices: Vec<Device>,
    pub device_tags: Vec<BackupDeviceTag>,
    pub access_groups: Vec<AccessGroup>,
    pub access_group_memberships: Vec<AccessGroupMembership>,
    pub device_visibility_grants: Vec<DeviceVisibilityGrant>,
    pub user_device_visibility_grants: Vec<UserDeviceVisibilityGrant>,
    pub address_books: Vec<AddressBook>,
    pub address_book_entries: Vec<AddressBookEntry>,
    pub server_config: Option<ServerConfig>,
    pub enrollment_tokens: Vec<BackupEnrollmentToken>,
    pub users: Vec<LegacyBackupUser>,
}

impl BackupDocumentV2 {
    pub(super) fn into_current(self) -> Result<BackupDocument, super::BackupValidationError> {
        if self.schema_version != 2 {
            return Err(super::BackupValidationError::UnsupportedSchemaVersion);
        }
        let document = BackupDocument {
            schema_version: BACKUP_SCHEMA_VERSION,
            exported_at: self.exported_at,
            sensitivity: self.sensitivity,
            sites: self.sites,
            tags: self.tags,
            devices: self.devices,
            device_tags: self.device_tags,
            access_groups: self.access_groups,
            access_group_memberships: self.access_group_memberships,
            device_visibility_grants: self.device_visibility_grants,
            user_device_visibility_grants: self.user_device_visibility_grants,
            address_books: self
                .address_books
                .into_iter()
                .map(|book| super::BackupAddressBook {
                    address_book_uuid: book.address_book_uuid,
                    owner_user_uuid: book.owner_user_uuid,
                    name: book.name,
                    book_kind: "personal".to_string(),
                })
                .collect(),
            address_book_access_rules: Vec::new(),
            address_book_tags: Vec::new(),
            address_book_entries: self.address_book_entries,
            address_book_entry_tags: Vec::new(),
            server_config: self.server_config,
            enrollment_tokens: self.enrollment_tokens,
            users: self
                .users
                .into_iter()
                .map(LegacyBackupUser::into_current)
                .collect(),
        };
        validate_backup_document(&document)?;
        Ok(document)
    }
}

impl BackupDocumentV1 {
    pub(super) fn into_current(self) -> Result<BackupDocument, super::BackupValidationError> {
        if self.schema_version != 1 {
            return Err(super::BackupValidationError::UnsupportedSchemaVersion);
        }
        let document = BackupDocument {
            schema_version: BACKUP_SCHEMA_VERSION,
            exported_at: self.exported_at,
            sensitivity: self.sensitivity,
            sites: self.sites,
            tags: self.tags,
            devices: self.devices,
            device_tags: self.device_tags,
            access_groups: Vec::new(),
            access_group_memberships: Vec::new(),
            device_visibility_grants: Vec::new(),
            user_device_visibility_grants: Vec::new(),
            address_books: Vec::new(),
            address_book_access_rules: Vec::new(),
            address_book_tags: Vec::new(),
            address_book_entries: Vec::new(),
            address_book_entry_tags: Vec::new(),
            server_config: self.server_config,
            enrollment_tokens: self.enrollment_tokens,
            users: self
                .users
                .into_iter()
                .map(LegacyBackupUser::into_current)
                .collect(),
        };
        validate_backup_document(&document)?;
        Ok(document)
    }
}
