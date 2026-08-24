//! Strict owner-approved migration manifest boundary.
//!
//! This module validates report-only migration preconditions and contains no
//! persistence or apply operation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use super::{migration_export::find_sensitive_field, RustDeskProImportDocument};

/// Versioned owner-approved instructions for a migration review. This is a
/// validation boundary only: it contains no source names, credentials, or
/// inferred matches, and it has no persistence counterpart.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApprovedMigrationManifest {
    pub schema_version: u32,
    pub source_snapshot_sha256: String,
    pub expected_counts: ManifestExpectedCounts,
    pub dry_run_report_sha256: String,
    pub approver: String,
    pub approved_at: String,
    pub expires_at: String,
    pub dispositions: Vec<MigrationDisposition>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestExpectedCounts {
    pub users: u64,
    pub groups: u64,
    pub devices: u64,
    pub address_books: u64,
    pub address_book_entries: u64,
    pub cross_group_edges: u64,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ManifestSourceKind {
    User,
    Group,
    Device,
    AddressBook,
    AddressBookEntry,
    CrossGroupEdge,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManifestAction {
    Import,
    Map,
    Merge,
    Retire,
    Defer,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCollisionDisposition {
    Block,
    MatchExisting,
    CreateNew,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RolePath {
    Ignore,
    MapIfCanonical,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialPath {
    Exclude,
    Reject,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityIntent {
    Unmapped,
    ExplicitScopeMapping,
    Visible,
    Hidden,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationDisposition {
    pub source_kind: ManifestSourceKind,
    #[serde(default)]
    pub source_parent_id: Option<String>,
    pub source_id: String,
    #[serde(deserialize_with = "deserialize_canonical_target_uuid")]
    pub target_uuid: Option<Uuid>,
    pub action: ManifestAction,
    pub identity_collision: IdentityCollisionDisposition,
    pub role_path: Option<RolePath>,
    pub credential_path: Option<CredentialPath>,
    pub visibility_intent: VisibilityIntent,
}

fn deserialize_canonical_target_uuid<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|value| {
            let uuid = Uuid::parse_str(&value)
                .map_err(|_| serde::de::Error::custom("target_uuid must be a UUID"))?;
            if uuid.to_string() != value {
                return Err(serde::de::Error::custom(
                    "target_uuid must be a canonical lowercase UUID",
                ));
            }
            Ok(uuid)
        })
        .transpose()
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestValidationError {
    #[error("unsupported approved-manifest schema version")]
    UnsupportedSchemaVersion,
    #[error("{field} must be a lowercase SHA-256 digest")]
    InvalidDigest { field: &'static str },
    #[error("approver must be non-empty and contain no control characters")]
    InvalidApprover,
    #[error("manifest timestamp is not canonical RFC3339: {field}")]
    InvalidTimestamp { field: &'static str },
    #[error("manifest expires_at must be after approved_at")]
    ExpiryBeforeApproval,
    #[error("manifest must contain at least one disposition")]
    EmptyDispositions,
    #[error("manifest contains duplicate source disposition")]
    DuplicateDisposition,
    #[error("target_uuid is required for this action")]
    TargetRequired,
    #[error("target_uuid is forbidden for retire and defer actions")]
    TargetForbidden,
    #[error("role_path and credential_path are only valid for user dispositions")]
    UserOnlyField,
    #[error("user dispositions require role_path and credential_path")]
    MissingUserPath,
    #[error("source_id must be non-empty and contain no control characters")]
    InvalidSourceId,
    #[error("source_parent_id is required only for address-book entries")]
    InvalidSourceParent,
}

pub const APPROVED_MIGRATION_MANIFEST_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestSourceCoverageError {
    #[error("manifest dispositions do not exactly cover every source entity")]
    Incomplete,
}

impl ApprovedMigrationManifest {
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.schema_version != APPROVED_MIGRATION_MANIFEST_SCHEMA_VERSION {
            return Err(ManifestValidationError::UnsupportedSchemaVersion);
        }
        validate_sha256(&self.source_snapshot_sha256, "source_snapshot_sha256")?;
        validate_sha256(&self.dry_run_report_sha256, "dry_run_report_sha256")?;
        if self.approver.trim().is_empty() || self.approver.chars().any(char::is_control) {
            return Err(ManifestValidationError::InvalidApprover);
        }
        let approved_at = parse_manifest_timestamp(&self.approved_at, "approved_at")?;
        let expires_at = parse_manifest_timestamp(&self.expires_at, "expires_at")?;
        if expires_at <= approved_at {
            return Err(ManifestValidationError::ExpiryBeforeApproval);
        }
        if self.dispositions.is_empty() {
            return Err(ManifestValidationError::EmptyDispositions);
        }
        let mut seen = HashSet::new();
        for disposition in &self.dispositions {
            if disposition.source_id.trim().is_empty()
                || disposition.source_id.chars().any(char::is_control)
            {
                return Err(ManifestValidationError::InvalidSourceId);
            }
            if disposition.source_kind == ManifestSourceKind::AddressBookEntry {
                if disposition
                    .source_parent_id
                    .as_deref()
                    .is_none_or(str::is_empty)
                    || disposition
                        .source_parent_id
                        .as_deref()
                        .is_some_and(|value| value.chars().any(char::is_control))
                {
                    return Err(ManifestValidationError::InvalidSourceParent);
                }
            } else if disposition.source_parent_id.is_some() {
                return Err(ManifestValidationError::InvalidSourceParent);
            }
            if !seen.insert((
                disposition.source_kind,
                disposition.source_parent_id.as_deref(),
                disposition.source_id.as_str(),
            )) {
                return Err(ManifestValidationError::DuplicateDisposition);
            }
            match disposition.action {
                ManifestAction::Import | ManifestAction::Map | ManifestAction::Merge => {
                    if disposition.target_uuid.is_none() {
                        return Err(ManifestValidationError::TargetRequired);
                    }
                }
                ManifestAction::Retire | ManifestAction::Defer => {
                    if disposition.target_uuid.is_some() {
                        return Err(ManifestValidationError::TargetForbidden);
                    }
                }
            }
            if disposition.source_kind == ManifestSourceKind::User {
                if disposition.role_path.is_none() || disposition.credential_path.is_none() {
                    return Err(ManifestValidationError::MissingUserPath);
                }
            } else if disposition.role_path.is_some() || disposition.credential_path.is_some() {
                return Err(ManifestValidationError::UserOnlyField);
            }
        }
        Ok(())
    }

    pub fn validate_against_source(
        &self,
        document: &RustDeskProImportDocument,
    ) -> Result<(), ManifestSourceCoverageError> {
        let mut required: HashSet<(ManifestSourceKind, Option<String>, String)> = HashSet::new();
        required.extend(
            document
                .users
                .iter()
                .map(|item| (ManifestSourceKind::User, None, item.user_id.clone())),
        );
        required.extend(
            document
                .groups
                .iter()
                .map(|item| (ManifestSourceKind::Group, None, item.group_id.clone())),
        );
        required.extend(
            document
                .devices
                .iter()
                .map(|item| (ManifestSourceKind::Device, None, item.rustdesk_id.clone())),
        );
        required.extend(document.address_books.iter().map(|item| {
            (
                ManifestSourceKind::AddressBook,
                None,
                item.address_book_id.clone(),
            )
        }));
        required.extend(document.address_book_entries.iter().map(|item| {
            (
                ManifestSourceKind::AddressBookEntry,
                Some(item.address_book_id.clone()),
                item.rustdesk_id.clone(),
            )
        }));
        required.extend(document.cross_group_edges.iter().map(|item| {
            (
                ManifestSourceKind::CrossGroupEdge,
                None,
                item.edge_id.clone(),
            )
        }));
        let covered: HashSet<_> = self
            .dispositions
            .iter()
            .map(|item| {
                (
                    item.source_kind,
                    item.source_parent_id.clone(),
                    item.source_id.clone(),
                )
            })
            .collect();
        if required.len() != covered.len() || required.iter().any(|item| !covered.contains(item)) {
            return Err(ManifestSourceCoverageError::Incomplete);
        }
        Ok(())
    }
}

#[path = "migration_manifest_semantics.rs"]
mod migration_manifest_semantics;
pub use migration_manifest_semantics::{planned_import_target, ManifestSemanticError};

fn validate_sha256(value: &str, field: &'static str) -> Result<(), ManifestValidationError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(ManifestValidationError::InvalidDigest { field });
    }
    Ok(())
}
fn parse_manifest_timestamp(
    value: &str,
    field: &'static str,
) -> Result<OffsetDateTime, ManifestValidationError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| ManifestValidationError::InvalidTimestamp { field })?;
    if parsed.format(&Rfc3339).ok().as_deref() != Some(value) {
        return Err(ManifestValidationError::InvalidTimestamp { field });
    }
    Ok(parsed)
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestParseError {
    #[error("invalid approved migration manifest JSON: {0}")]
    InvalidJson(String),
    #[error("sensitive field rejected at {path}")]
    SensitiveField { path: String },
    #[error("invalid approved migration manifest: {0}")]
    InvalidManifest(String),
}

pub fn parse_approved_migration_manifest_json(
    value: &str,
) -> Result<ApprovedMigrationManifest, ManifestParseError> {
    let json: serde_json::Value = serde_json::from_str(value)
        .map_err(|error| ManifestParseError::InvalidJson(error.to_string()))?;
    if let Some(path) = find_sensitive_field(&json, "$".to_string()) {
        return Err(ManifestParseError::SensitiveField { path });
    }
    let manifest: ApprovedMigrationManifest = serde_json::from_value(json)
        .map_err(|error| ManifestParseError::InvalidJson(error.to_string()))?;
    manifest
        .validate()
        .map_err(|error| ManifestParseError::InvalidManifest(error.to_string()))?;
    Ok(manifest)
}

pub fn sha256_digest(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    hex::encode(digest.finalize())
}

pub fn expected_import_counts(document: &RustDeskProImportDocument) -> ManifestExpectedCounts {
    ManifestExpectedCounts {
        users: document.users.len() as u64,
        groups: document.groups.len() as u64,
        devices: document.devices.len() as u64,
        address_books: document.address_books.len() as u64,
        address_book_entries: document.address_book_entries.len() as u64,
        cross_group_edges: document.cross_group_edges.len() as u64,
    }
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestPreconditionError {
    #[error("source snapshot digest does not match manifest")]
    SourceDigestMismatch,
    #[error("expected source counts do not match manifest")]
    ExpectedCountsMismatch,
    #[error("dry-run report digest does not match manifest")]
    DryRunReportDigestMismatch,
    #[error("manifest is not effective yet")]
    NotYetApproved,
    #[error("manifest has expired")]
    Expired,
}

pub fn validate_manifest_preconditions(
    manifest: &ApprovedMigrationManifest,
    source_snapshot: &[u8],
    source_document: &RustDeskProImportDocument,
    dry_run_report_json: &[u8],
    now: OffsetDateTime,
) -> Result<(), ManifestPreconditionError> {
    if sha256_digest(source_snapshot) != manifest.source_snapshot_sha256 {
        return Err(ManifestPreconditionError::SourceDigestMismatch);
    }
    if expected_import_counts(source_document) != manifest.expected_counts {
        return Err(ManifestPreconditionError::ExpectedCountsMismatch);
    }
    if sha256_digest(dry_run_report_json) != manifest.dry_run_report_sha256 {
        return Err(ManifestPreconditionError::DryRunReportDigestMismatch);
    }
    let approved_at = parse_manifest_timestamp(&manifest.approved_at, "approved_at")
        .expect("validated manifest has canonical approval timestamp");
    if now < approved_at {
        return Err(ManifestPreconditionError::NotYetApproved);
    }
    let expires_at = parse_manifest_timestamp(&manifest.expires_at, "expires_at")
        .expect("validated manifest has canonical expiry");
    if now >= expires_at {
        return Err(ManifestPreconditionError::Expired);
    }
    Ok(())
}
