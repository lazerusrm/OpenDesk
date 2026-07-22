use serde::{Deserialize, Serialize};

use super::{
    validate_backup_document, BackupDeviceTag, BackupDocument, BackupEnrollmentToken,
    BackupSensitivity, BackupUser, BACKUP_SCHEMA_VERSION,
};
use crate::domain::{device::Device, server_config::ServerConfig, site::Site, tag::Tag};

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
    pub users: Vec<BackupUser>,
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
            address_book_entries: Vec::new(),
            server_config: self.server_config,
            enrollment_tokens: self.enrollment_tokens,
            users: self.users,
        };
        validate_backup_document(&document)?;
        Ok(document)
    }
}
