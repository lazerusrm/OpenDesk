use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::device::Device;
use super::role::Role;
use super::server_config::{validate_server_config, ServerConfig};
use super::site::Site;
use super::tag::Tag;

pub const BACKUP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
pub struct BackupDeviceTag {
    pub device_uuid: Uuid,
    pub tag_uuid: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupUser {
    pub user_uuid: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupEnrollmentToken {
    pub enrollment_token_uuid: Uuid,
    pub token_hash: String,
    pub label: String,
    pub site_uuid: Option<Uuid>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupDocument {
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
    #[error("backup contains an invalid user role")]
    InvalidRole,
    #[error("backup contains an invalid server configuration")]
    InvalidServerConfig,
}

pub fn validate_backup_document(document: &BackupDocument) -> Result<(), BackupValidationError> {
    if document.schema_version != BACKUP_SCHEMA_VERSION {
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
    unique_ids(
        document
            .enrollment_tokens
            .iter()
            .map(|token| token.enrollment_token_uuid),
        "enrollment token",
    )?;
    unique_ids(document.users.iter().map(|user| user.user_uuid), "user")?;

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

pub fn render_backup_json(document: &BackupDocument) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(document)
}

pub fn parse_backup_json(value: &str) -> Result<BackupDocument, serde_json::Error> {
    serde_json::from_str(value)
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
            server_config: None,
            enrollment_tokens: vec![],
            users: vec![BackupUser {
                user_uuid: Uuid::new_v4(),
                username: "admin".to_string(),
                password_hash: "hash".to_string(),
                role: "admin".to_string(),
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
