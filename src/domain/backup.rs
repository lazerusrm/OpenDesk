use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::device::Device;
use super::server_config::ServerConfig;
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BackupValidationError {
    #[error("unsupported backup schema version")]
    UnsupportedSchemaVersion,
    #[error("backup must include at least one user")]
    EmptyUsers,
}

pub fn validate_backup_document(document: &BackupDocument) -> Result<(), BackupValidationError> {
    if document.schema_version != BACKUP_SCHEMA_VERSION {
        return Err(BackupValidationError::UnsupportedSchemaVersion);
    }
    if document.users.is_empty() {
        return Err(BackupValidationError::EmptyUsers);
    }
    Ok(())
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
}