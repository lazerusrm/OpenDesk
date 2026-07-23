//! Sanitized migration contract shared by export review and a future apply worker.
//!
//! This module is deliberately data-only. It contains no repository, storage, or
//! remote action. Credentials are represented only by reset policy; hashes and
//! secret values are not valid members of this contract.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub use crate::domain::migration_contract_validation::validate_export;
use crate::domain::migration_contract_validation::{
    find_sensitive_field, validate_text, validate_timestamp,
};

pub const SANITIZED_MIGRATION_SCHEMA_VERSION: u32 = 2;

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
    pub cross_group_access: Vec<SourceCrossGroupAccess>,
    pub devices: Vec<SourceDevice>,
    pub address_books: Vec<SourceAddressBook>,
    pub address_book_entries: Vec<SourceAddressBookEntry>,
    pub settings: Vec<SourceSetting>,
    #[serde(default)]
    pub unsupported_semantics: Vec<UnsupportedSourceSemantics>,
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
    pub allow_device_access_within_group: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct SourceCrossGroupAccess {
    pub source_group_id: String,
    pub target_group_id: String,
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
    pub owner_source_user_id: Option<String>,
    pub source_group_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceAddressBook {
    pub source_address_book_id: String,
    pub name: String,
    pub owner_source_user_id: Option<String>,
    pub book_kind: AddressBookKind,
    pub rules: Vec<SourceAddressBookRule>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AddressBookKind {
    Personal,
    Shared,
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

/// An unsupported source category is never transferred. Its count is retained
/// only when a signed migration explicitly retires that category.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedSourceSemantics {
    pub category: UnsupportedSemanticsCategory,
    pub count: u64,
    pub disposition: UnsupportedSemanticsDisposition,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedSemanticsCategory {
    AddressBookNotes,
    CustomClient,
    Strategy,
    Settings,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedSemanticsDisposition {
    Retired,
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
    #[error("device references an unknown access group")]
    UnknownDeviceGroup,
    #[error("device references an unknown owner")]
    UnknownDeviceOwner,
    #[error("cross-group access references an unknown group")]
    UnknownCrossGroupReference,
    #[error("source collection contains duplicate cross-group access")]
    DuplicateCrossGroupAccess,
    #[error("address-book entry references an unknown device")]
    UnknownAddressBookDevice,
    #[error("address-book rule contains a duplicate principal/permission")]
    DuplicateAddressBookRule,
    #[error("source collection contains a duplicate identifier: {field}")]
    DuplicateSourceIdentifier { field: &'static str },
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
