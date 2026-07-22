//! Sanitized migration contract shared by export review and a future apply worker.
//!
//! This module is deliberately data-only. It contains no repository, storage, or
//! remote action. Credentials are represented only by reset policy; hashes and
//! secret values are not valid members of this contract.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub const SANITIZED_MIGRATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationRunStatus {
    Exported,
    Validated,
    DryRunReady,
    Approved,
    Applied,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceProvenance {
    pub source_system: String,
    pub source_instance: String,
    pub source_export_id: String,
    pub source_schema_version: String,
    pub exported_at: String,
    pub snapshot_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationRunState {
    pub run_id: Uuid,
    pub status: MigrationRunStatus,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub source_snapshot_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SanitizedMigrationExport {
    pub schema_version: u32,
    pub provenance: SourceProvenance,
    pub run: MigrationRunState,
    pub users: Vec<SourceUser>,
    pub groups: Vec<SourceGroup>,
    pub user_group_memberships: Vec<SourceUserGroupMembership>,
    pub devices: Vec<SourceDevice>,
    pub address_books: Vec<SourceAddressBook>,
    pub address_book_entries: Vec<SourceAddressBookEntry>,
    pub settings: Vec<SourceSetting>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceUser {
    pub source_user_id: String,
    pub username: String,
    pub role: Option<String>,
    pub credential_reset_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceGroup {
    pub source_group_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct SourceUserGroupMembership {
    pub source_user_id: String,
    pub source_group_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceDevice {
    pub rustdesk_id: String,
    pub alias: String,
    pub hostname: Option<String>,
    pub source_group_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAddressBook {
    pub source_address_book_id: String,
    pub name: String,
    pub owner_source_user_id: Option<String>,
    pub rules: Vec<SourceAddressBookRule>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AddressBookPrincipalType {
    User,
    Group,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AddressBookPermission {
    Read,
    Write,
    Admin,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAddressBookRule {
    pub principal_type: AddressBookPrincipalType,
    pub principal_id: String,
    pub permission: AddressBookPermission,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAddressBookEntry {
    pub source_address_book_id: String,
    pub rustdesk_id: String,
    pub alias: String,
    pub notes: Option<String>,
    pub credential_reset_required: bool,
}

/// A setting has no source value. The disposition must be explicit so an apply
/// worker cannot accidentally carry forward an undocumented security setting.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettingsDisposition {
    Exclude,
    ManualReview,
    Map,
    Retire,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSetting {
    pub key: String,
    pub disposition: SettingsDisposition,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationManifest {
    pub schema_version: u32,
    pub run_id: Uuid,
    pub source_snapshot_sha256: String,
    pub credential_reset_required: bool,
    pub settings_disposition: SettingsDisposition,
    pub approved_by: String,
    pub approved_at: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MigrationContractError {
    #[error("unsupported sanitized migration schema version")]
    UnsupportedSchemaVersion,
    #[error("invalid or mismatched source snapshot digest")]
    InvalidSnapshotDigest,
    #[error("invalid source value: {field}")]
    InvalidSourceValue { field: &'static str },
    #[error("source user-group membership references an unknown user or group")]
    UnknownMembershipReference,
    #[error("address-book rule contains a duplicate principal/permission")]
    DuplicateAddressBookRule,
    #[error("address-book references an unknown owner")]
    UnknownAddressBookOwner,
    #[error("address-book entry references an unknown address book")]
    UnknownAddressBook,
    #[error("credential reset is required for every imported credential-bearing record")]
    CredentialResetRequired,
    #[error("manifest credential reset requirement must be true")]
    ManifestCredentialResetNotRequired,
    #[error("migration manifest run id does not match export run")]
    ManifestRunMismatch,
    #[error("migration manifest source digest does not match export")]
    ManifestSourceMismatch,
    #[error("invalid migration timestamp: {field}")]
    InvalidTimestamp { field: &'static str },
}

pub fn validate_manifest(
    manifest: &MigrationManifest,
    export: &SanitizedMigrationExport,
) -> Result<(), MigrationContractError> {
    if manifest.schema_version != SANITIZED_MIGRATION_SCHEMA_VERSION {
        return Err(MigrationContractError::UnsupportedSchemaVersion);
    }
    if manifest.run_id != export.run.run_id {
        return Err(MigrationContractError::ManifestRunMismatch);
    }
    if manifest.source_snapshot_sha256 != export.provenance.snapshot_sha256 {
        return Err(MigrationContractError::ManifestSourceMismatch);
    }
    if !manifest.credential_reset_required {
        return Err(MigrationContractError::ManifestCredentialResetNotRequired);
    }
    validate_text(&manifest.approved_by, "approved_by")?;
    validate_timestamp(&manifest.approved_at, "approved_at")?;
    validate_export(export)
}

pub fn validate_export(document: &SanitizedMigrationExport) -> Result<(), MigrationContractError> {
    if document.schema_version != SANITIZED_MIGRATION_SCHEMA_VERSION {
        return Err(MigrationContractError::UnsupportedSchemaVersion);
    }
    validate_provenance(&document.provenance)?;
    validate_run(&document.run)?;
    if document.provenance.snapshot_sha256 != document.run.source_snapshot_sha256 {
        return Err(MigrationContractError::InvalidSnapshotDigest);
    }
    let users: HashSet<&str> = document
        .users
        .iter()
        .map(|item| item.source_user_id.as_str())
        .collect();
    let groups: HashSet<&str> = document
        .groups
        .iter()
        .map(|item| item.source_group_id.as_str())
        .collect();
    for item in &document.users {
        validate_text(&item.source_user_id, "source_user_id")?;
        validate_text(&item.username, "username")?;
        if !item.credential_reset_required {
            return Err(MigrationContractError::CredentialResetRequired);
        }
    }
    for item in &document.user_group_memberships {
        if !users.contains(item.source_user_id.as_str())
            || !groups.contains(item.source_group_id.as_str())
        {
            return Err(MigrationContractError::UnknownMembershipReference);
        }
    }
    let books: HashSet<&str> = document
        .address_books
        .iter()
        .map(|item| item.source_address_book_id.as_str())
        .collect();
    for book in &document.address_books {
        validate_text(&book.source_address_book_id, "source_address_book_id")?;
        if let Some(owner) = &book.owner_source_user_id {
            if !users.contains(owner.as_str()) {
                return Err(MigrationContractError::UnknownAddressBookOwner);
            }
        }
        let mut rules = HashSet::new();
        for rule in &book.rules {
            validate_text(&rule.principal_id, "principal_id")?;
            if !rules.insert((
                rule.principal_type,
                rule.principal_id.as_str(),
                rule.permission,
            )) {
                return Err(MigrationContractError::DuplicateAddressBookRule);
            }
            let valid = match rule.principal_type {
                AddressBookPrincipalType::User => users.contains(rule.principal_id.as_str()),
                AddressBookPrincipalType::Group => groups.contains(rule.principal_id.as_str()),
            };
            if !valid {
                return Err(MigrationContractError::UnknownMembershipReference);
            }
        }
    }
    for entry in &document.address_book_entries {
        if !books.contains(entry.source_address_book_id.as_str()) {
            return Err(MigrationContractError::UnknownAddressBook);
        }
        if !entry.credential_reset_required {
            return Err(MigrationContractError::CredentialResetRequired);
        }
    }
    Ok(())
}

fn validate_provenance(value: &SourceProvenance) -> Result<(), MigrationContractError> {
    validate_text(&value.source_system, "source_system")?;
    validate_text(&value.source_instance, "source_instance")?;
    validate_text(&value.source_export_id, "source_export_id")?;
    validate_text(&value.source_schema_version, "source_schema_version")?;
    validate_digest(&value.snapshot_sha256)
}

fn validate_run(value: &MigrationRunState) -> Result<(), MigrationContractError> {
    validate_timestamp(&value.started_at, "started_at")?;
    if let Some(completed) = &value.completed_at {
        validate_timestamp(completed, "completed_at")?;
    }
    validate_digest(&value.source_snapshot_sha256)
}

fn validate_text(value: &str, field: &'static str) -> Result<(), MigrationContractError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(MigrationContractError::InvalidSourceValue { field });
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), MigrationContractError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(MigrationContractError::InvalidSnapshotDigest);
    }
    Ok(())
}

fn validate_timestamp(value: &str, field: &'static str) -> Result<(), MigrationContractError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| MigrationContractError::InvalidTimestamp { field })?;
    if parsed.format(&Rfc3339).ok().as_deref() != Some(value) {
        return Err(MigrationContractError::InvalidTimestamp { field });
    }
    Ok(())
}

pub fn parse_sanitized_export(
    value: &str,
) -> Result<SanitizedMigrationExport, MigrationContractError> {
    let json: serde_json::Value = serde_json::from_str(value)
        .map_err(|_| MigrationContractError::InvalidSourceValue { field: "json" })?;
    if let Some(path) = find_sensitive_field(&json, "$") {
        tracing::debug!(path, "sanitized migration export rejected sensitive field");
        return Err(MigrationContractError::InvalidSourceValue { field: "sensitive" });
    }
    let document: SanitizedMigrationExport = serde_json::from_value(json)
        .map_err(|_| MigrationContractError::InvalidSourceValue { field: "schema" })?;
    validate_export(&document)?;
    Ok(document)
}

pub fn sha256_digest(value: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(value);
    hex::encode(digest.finalize())
}

fn find_sensitive_field(value: &serde_json::Value, path: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(fields) => fields.iter().find_map(|(name, value)| {
            if is_sensitive_field(name) {
                Some(format!("{path}.{name}"))
            } else {
                find_sensitive_field(value, &format!("{path}.{name}"))
            }
        }),
        serde_json::Value::Array(values) => values
            .iter()
            .enumerate()
            .find_map(|(index, value)| find_sensitive_field(value, &format!("{path}[{index}]"))),
        _ => None,
    }
}

fn is_sensitive_field(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    if name == "credential_reset_required" {
        return false;
    }
    let compact = name.replace(['_', '-'], "");
    name.contains("password")
        || name.contains("secret")
        || name.contains("credential")
        || name.contains("private")
        || name.contains("hash")
        || compact.contains("privatekey")
        || name.ends_with("_token")
        || name == "token"
        || name == "api_key"
}
