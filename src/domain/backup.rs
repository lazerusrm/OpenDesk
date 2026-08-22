use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::access_group::AccessGroup;
use super::access_group_membership::AccessGroupMembership;
use super::address_book::AddressBookEntry;
use super::device::Device;
use super::device_visibility::{
    AccessGroupAccessGrant, DeviceVisibilityGrant, UserDeviceVisibilityGrant,
};
use super::server_config::ServerConfig;
use super::site::Site;
use super::tag::Tag;

#[path = "backup_scoped_validation.rs"]
mod backup_scoped_validation;
#[path = "backup_v1_conversion.rs"]
mod backup_v1_conversion;
use backup_v1_conversion::{BackupDocumentV1, BackupDocumentV2};

pub const BACKUP_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupSensitivity {
    pub contains_password_hashes: bool,
    pub contains_enrollment_token_hashes: bool,
    pub excludes_sessions: bool,
    pub excludes_audit_events: bool,
    pub excludes_endpoint_checkins: bool,
}

impl Default for BackupSensitivity {
    fn default() -> Self {
        Self {
            contains_password_hashes: true,
            contains_enrollment_token_hashes: true,
            excludes_sessions: true,
            excludes_audit_events: true,
            excludes_endpoint_checkins: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupDeviceTag {
    pub device_uuid: Uuid,
    pub tag_uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupUser {
    pub user_uuid: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub activation_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupEnrollmentToken {
    pub enrollment_token_uuid: Uuid,
    pub token_hash: String,
    pub label: String,
    pub site_uuid: Option<Uuid>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupAddressBook {
    pub address_book_uuid: Uuid,
    pub owner_user_uuid: Uuid,
    pub name: String,
    pub book_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupAddressBookAccessRule {
    pub address_book_uuid: Uuid,
    pub principal_type: String,
    pub principal_uuid: Uuid,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupAddressBookTag {
    pub address_book_uuid: Uuid,
    pub name: String,
    pub color: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupAddressBookEntryTag {
    pub address_book_entry_uuid: Uuid,
    pub address_book_uuid: Uuid,
    pub tag_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackupDocument {
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
    #[serde(default)]
    pub access_group_access_grants: Vec<AccessGroupAccessGrant>,
    pub address_books: Vec<BackupAddressBook>,
    pub address_book_access_rules: Vec<BackupAddressBookAccessRule>,
    pub address_book_tags: Vec<BackupAddressBookTag>,
    pub address_book_entries: Vec<AddressBookEntry>,
    pub address_book_entry_tags: Vec<BackupAddressBookEntryTag>,
    pub server_config: Option<ServerConfig>,
    pub enrollment_tokens: Vec<BackupEnrollmentToken>,
    pub users: Vec<BackupUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupReadiness {
    pub status: &'static str,
    pub schedule_configured: bool,
    pub destination_configured: bool,
    pub execution: &'static str,
}

pub fn backup_readiness(
    schedule_configured: bool,
    destination_configured: bool,
) -> BackupReadiness {
    let status = match (schedule_configured, destination_configured) {
        (true, true) => "configured_external_runner",
        (true, false) => "incomplete",
        _ => "manual_only",
    };
    BackupReadiness {
        status,
        schedule_configured,
        destination_configured,
        execution: "external_runner_required",
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BackupValidationError {
    #[error("unsupported backup schema version")]
    UnsupportedSchemaVersion,
    #[error("backup must include at least one user")]
    EmptyUsers,
    #[error("backup contains duplicate {collection} identifier")]
    DuplicateIdentifier { collection: &'static str },
    #[error("backup contains an invalid {relation} reference")]
    InvalidReference { relation: &'static str },
    #[error("backup contains an invalid {field}")]
    InvalidValue { field: &'static str },
    #[error("backup contains a noncanonical {field}")]
    NonCanonicalValue { field: &'static str },
    #[error("backup contains an invalid user role")]
    InvalidRole,
    #[error("backup contains an invalid server configuration")]
    InvalidServerConfig,
}

pub use backup_scoped_validation::validate_backup_document;

pub fn render_backup_json(document: &BackupDocument) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(document)
}

/// Parse released backups and convert older shapes at this external boundary.
/// Current exports and restores are always v3; v1 and v2 conversions add only
/// explicitly absent fields before validating the current document.
pub fn parse_backup_json(value: &str) -> Result<BackupDocument, serde_json::Error> {
    let raw: serde_json::Value = serde_json::from_str(value)?;
    let version = raw
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "backup schema_version is required",
            ))
        })? as u32;
    match version {
        1 => serde_json::from_value::<BackupDocumentV1>(raw)?
            .into_current()
            .map_err(backup_validation_error),
        2 => serde_json::from_value::<BackupDocumentV2>(raw)?
            .into_current()
            .map_err(backup_validation_error),
        BACKUP_SCHEMA_VERSION => {
            let document: BackupDocument = serde_json::from_value(raw)?;
            validate_backup_document(&document).map_err(backup_validation_error)?;
            Ok(document)
        }
        _ => Err(backup_validation_error(
            BackupValidationError::UnsupportedSchemaVersion,
        )),
    }
}

fn backup_validation_error(error: BackupValidationError) -> serde_json::Error {
    serde_json::Error::io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_document() -> BackupDocument {
        BackupDocument {
            schema_version: BACKUP_SCHEMA_VERSION,
            exported_at: "2026-06-24T12:00:00Z".to_string(),
            sensitivity: BackupSensitivity::default(),
            sites: vec![],
            tags: vec![],
            devices: vec![],
            device_tags: vec![],
            access_groups: vec![],
            access_group_memberships: vec![],
            device_visibility_grants: vec![],
            user_device_visibility_grants: vec![],
            access_group_access_grants: vec![],
            address_books: vec![],
            address_book_access_rules: vec![],
            address_book_tags: vec![],
            address_book_entries: vec![],
            address_book_entry_tags: vec![],
            server_config: None,
            enrollment_tokens: vec![],
            users: vec![BackupUser {
                user_uuid: Uuid::new_v4(),
                username: "admin".to_string(),
                password_hash: "hash".to_string(),
                role: "admin".to_string(),
                activation_state: "active".to_string(),
            }],
        }
    }

    #[test]
    fn backup_json_round_trip_preserves_document() {
        let document = sample_document();
        let json = render_backup_json(&document).expect("serialize");
        let parsed = parse_backup_json(&json).expect("parse");
        assert_eq!(parsed, document);
    }

    #[test]
    fn v3_backup_without_access_group_access_grants_defaults_empty() {
        let document = sample_document();
        let mut raw: serde_json::Value =
            serde_json::from_str(&render_backup_json(&document).expect("serialize")).unwrap();
        raw.as_object_mut()
            .expect("object")
            .remove("access_group_access_grants");
        let parsed = parse_backup_json(&raw.to_string()).expect("parse");
        assert!(parsed.access_group_access_grants.is_empty());
    }

    #[test]
    fn parse_backup_json_converts_v1_boundary_document() {
        let document = sample_document();
        let v1 = BackupDocumentV1 {
            schema_version: 1,
            exported_at: document.exported_at.clone(),
            sensitivity: document.sensitivity.clone(),
            sites: document.sites.clone(),
            tags: document.tags.clone(),
            devices: document.devices.clone(),
            device_tags: document.device_tags.clone(),
            server_config: document.server_config.clone(),
            enrollment_tokens: document.enrollment_tokens.clone(),
            users: document
                .users
                .iter()
                .cloned()
                .map(|user| backup_v1_conversion::LegacyBackupUser {
                    user_uuid: user.user_uuid,
                    username: user.username,
                    password_hash: user.password_hash,
                    role: user.role,
                })
                .collect(),
        };
        let json = serde_json::to_string(&v1).expect("serialize v1 backup");
        let converted = parse_backup_json(&json).expect("convert v1 backup");
        assert_eq!(converted.schema_version, BACKUP_SCHEMA_VERSION);
        assert!(converted.access_groups.is_empty());
        assert!(converted.address_books.is_empty());
        assert_eq!(converted.users, document.users);
    }

    #[test]
    fn validate_backup_document_rejects_unknown_schema() {
        let mut document = sample_document();
        document.schema_version = 99;
        assert_eq!(
            validate_backup_document(&document),
            Err(BackupValidationError::UnsupportedSchemaVersion)
        );
    }

    #[test]
    fn validate_backup_document_rejects_dangling_device_tag() {
        let mut document = sample_document();
        document.device_tags.push(BackupDeviceTag {
            device_uuid: Uuid::new_v4(),
            tag_uuid: Uuid::new_v4(),
        });
        assert_eq!(
            validate_backup_document(&document),
            Err(BackupValidationError::InvalidReference {
                relation: "device tag"
            })
        );
    }

    #[test]
    fn backup_readiness_requires_schedule_and_destination() {
        assert_eq!(
            backup_readiness(true, true).status,
            "configured_external_runner"
        );
        assert_eq!(backup_readiness(true, false).status, "incomplete");
        assert_eq!(backup_readiness(false, false).status, "manual_only");
    }
}
