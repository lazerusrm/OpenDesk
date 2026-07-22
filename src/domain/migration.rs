//! Dry-run-only adapter for sanitized RustDesk Pro exports.
//!
//! This boundary accepts only the fields needed to reconcile identities, groups,
//! devices, and address-book entries. It never writes to storage and deliberately
//! has no credential, session, key, or secret fields.

use serde::Deserialize;
use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

use super::{device::Device, site::Site, user::User};

pub const RUSTDESK_PRO_IMPORT_SCHEMA_VERSION: u32 = 1;

/// The external export shape is intentionally named at the integration boundary.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProImportDocument {
    pub schema_version: u32,
    pub users: Vec<RustDeskProUser>,
    pub groups: Vec<RustDeskProGroup>,
    pub devices: Vec<RustDeskProDevice>,
    pub address_books: Vec<RustDeskProAddressBook>,
    pub address_book_entries: Vec<RustDeskProAddressBookEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProUser {
    pub user_id: String,
    pub username: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProGroup {
    pub group_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProDevice {
    pub rustdesk_id: String,
    pub alias: String,
    pub hostname: Option<String>,
    pub group_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProAddressBook {
    pub address_book_id: String,
    pub name: String,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProAddressBookEntry {
    pub address_book_id: String,
    pub rustdesk_id: String,
    pub alias: String,
    pub notes: Option<String>,
}

/// Existing OpenDesk state plus operator-supplied scope semantics evidence.
/// A group is never treated as a site based on matching names.
#[derive(Debug, Clone, Default)]
pub struct MigrationSnapshot {
    pub users: Vec<User>,
    pub sites: Vec<Site>,
    pub devices: Vec<Device>,
    pub scope_site_mappings: Vec<ScopeSiteMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeSiteMapping {
    pub group_id: String,
    pub site_uuid: Uuid,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportParseError {
    #[error("invalid import JSON: {0}")]
    InvalidJson(String),
    #[error("sensitive field rejected at {path}")]
    SensitiveField { path: String },
    #[error("unsupported import schema version")]
    UnsupportedSchemaVersion,
}

/// Parse a sanitized export and reject sensitive fields before deserialization.
/// Unknown fields are rejected by `deny_unknown_fields` on every boundary type.
pub fn parse_rustdesk_pro_import_json(
    value: &str,
) -> Result<RustDeskProImportDocument, ImportParseError> {
    let json: serde_json::Value = serde_json::from_str(value)
        .map_err(|error| ImportParseError::InvalidJson(error.to_string()))?;
    if let Some(path) = find_sensitive_field(&json, "$".to_string()) {
        return Err(ImportParseError::SensitiveField { path });
    }
    let document: RustDeskProImportDocument = serde_json::from_value(json)
        .map_err(|error| ImportParseError::InvalidJson(error.to_string()))?;
    if document.schema_version != RUSTDESK_PRO_IMPORT_SCHEMA_VERSION {
        return Err(ImportParseError::UnsupportedSchemaVersion);
    }
    validate_external_references(&document)?;
    Ok(document)
}

fn validate_external_references(
    document: &RustDeskProImportDocument,
) -> Result<(), ImportParseError> {
    let groups: HashSet<&str> = document
        .groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect();
    for (index, device) in document.devices.iter().enumerate() {
        for (group_index, group_id) in device.group_ids.iter().enumerate() {
            if !groups.contains(group_id.as_str()) {
                return Err(ImportParseError::InvalidJson(format!(
                    "unknown group reference at $.devices[{index}].group_ids[{group_index}]"
                )));
            }
        }
    }
    for (index, book) in document.address_books.iter().enumerate() {
        if let Some(group_id) = book.group_id.as_deref() {
            if !groups.contains(group_id) {
                return Err(ImportParseError::InvalidJson(format!(
                    "unknown group reference at $.address_books[{index}].group_id"
                )));
            }
        }
    }
    let books: HashSet<&str> = document
        .address_books
        .iter()
        .map(|book| book.address_book_id.as_str())
        .collect();
    for (index, entry) in document.address_book_entries.iter().enumerate() {
        if !books.contains(entry.address_book_id.as_str()) {
            return Err(ImportParseError::InvalidJson(format!(
                "unknown address-book reference at $.address_book_entries[{index}].address_book_id"
            )));
        }
    }
    Ok(())
}

fn find_sensitive_field(value: &serde_json::Value, path: String) -> Option<String> {
    match value {
        serde_json::Value::Object(fields) => fields.iter().find_map(|(name, value)| {
            if is_sensitive_field(name) {
                Some(format!("{path}.{name}"))
            } else {
                find_sensitive_field(value, format!("{path}.{name}"))
            }
        }),
        serde_json::Value::Array(values) => values
            .iter()
            .enumerate()
            .find_map(|(index, value)| find_sensitive_field(value, format!("{path}[{index}]"))),
        _ => None,
    }
}

fn is_sensitive_field(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let compact = name.replace(['_', '-'], "");
    name.contains("password")
        || name.contains("credential")
        || compact.contains("privatekey")
        || name.contains("jwt")
        || name == "access_token"
        || name == "refresh_token"
        || name == "token"
        || name.ends_with("_token")
        || name.ends_with("_hash")
        || name == "secret"
        || name.contains("secret")
        || name == "api_key"
        || name.ends_with("_secret")
}

#[path = "migration_operator.rs"]
mod migration_operator;
pub use migration_operator::{
    load_migration_snapshot, parse_scope_site_mapping, validate_scope_site_mappings,
    ScopeMappingParseError, ScopeMappingValidationError, SnapshotLoadError,
};

#[path = "migration_reconciliation.rs"]
mod migration_reconciliation;
pub use migration_reconciliation::{
    dry_run_import, ImportDryRunReport, ReconciliationAction, ReconciliationItem,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::Device;

    fn document() -> RustDeskProImportDocument {
        RustDeskProImportDocument {
            schema_version: RUSTDESK_PRO_IMPORT_SCHEMA_VERSION,
            users: vec![RustDeskProUser {
                user_id: "pro-user-1".to_string(),
                username: "admin".to_string(),
                role: Some("administrator".to_string()),
            }],
            groups: vec![RustDeskProGroup {
                group_id: "group-1".to_string(),
                name: "Production".to_string(),
            }],
            devices: vec![RustDeskProDevice {
                rustdesk_id: "123".to_string(),
                alias: "Workstation".to_string(),
                hostname: Some("ws-1".to_string()),
                group_ids: vec!["group-1".to_string()],
            }],
            address_books: vec![RustDeskProAddressBook {
                address_book_id: "book-1".to_string(),
                name: "Operators".to_string(),
                group_id: Some("group-1".to_string()),
            }],
            address_book_entries: vec![RustDeskProAddressBookEntry {
                address_book_id: "book-1".to_string(),
                rustdesk_id: "123".to_string(),
                alias: "Workstation".to_string(),
                notes: None,
            }],
        }
    }

    fn snapshot() -> MigrationSnapshot {
        MigrationSnapshot {
            users: vec![User {
                user_uuid: Uuid::new_v4(),
                username: "admin".to_string(),
                role: "admin".to_string(),
            }],
            sites: vec![Site {
                site_uuid: Uuid::new_v4(),
                name: "Production".to_string(),
            }],
            devices: vec![Device {
                device_uuid: Uuid::new_v4(),
                rustdesk_id: Some("123".to_string()),
                alias: "Workstation".to_string(),
                hostname: None,
                os_family: None,
                os_version: None,
                architecture: None,
                rustdesk_version: None,
                site_uuid: None,
                owner: None,
                notes: None,
                archived: false,
                last_checkin_at: None,
            }],
            scope_site_mappings: vec![],
        }
    }

    #[test]
    fn parser_rejects_unknown_fields_at_nested_boundary() {
        let json = r#"{
            "schema_version": 1, "users": [], "groups": [], "devices": [],
            "address_books": [], "address_book_entries": [], "unexpected": true
        }"#;
        let error = parse_rustdesk_pro_import_json(json).expect_err("unknown field");
        assert!(
            matches!(error, ImportParseError::InvalidJson(message) if message.contains("unknown field"))
        );
    }

    #[test]
    fn parser_rejects_sensitive_fields_before_schema_deserialization() {
        let json = r#"{
            "schema_version": 1, "users": [{"user_id":"u", "username":"a", "role":null, "password_hash":"x"}],
            "groups": [], "devices": [], "address_books": [], "address_book_entries": []
        }"#;
        assert_eq!(
            parse_rustdesk_pro_import_json(json),
            Err(ImportParseError::SensitiveField {
                path: "$.users[0].password_hash".to_string()
            })
        );
    }

    #[test]
    fn parser_rejects_dangling_external_references() {
        let json = r#"{
            "schema_version": 1, "users": [],
            "groups": [{"group_id":"known", "name":"Known"}],
            "devices": [{"rustdesk_id":"1", "alias":"Device", "hostname":null, "group_ids":["missing"]}],
            "address_books": [{"address_book_id":"book", "name":"Book", "group_id":"known"}],
            "address_book_entries": [{"address_book_id":"missing-book", "rustdesk_id":"1", "alias":"Entry", "notes":null}]
        }"#;
        let error = parse_rustdesk_pro_import_json(json).expect_err("dangling reference");
        assert!(
            matches!(error, ImportParseError::InvalidJson(message) if message.contains("unknown group reference"))
        );

        let json = r#"{
            "schema_version": 1, "users": [],
            "groups": [{"group_id":"known", "name":"Known"}],
            "devices": [],
            "address_books": [{"address_book_id":"book", "name":"Book", "group_id":"known"}],
            "address_book_entries": [{"address_book_id":"missing-book", "rustdesk_id":"1", "alias":"Entry", "notes":null}]
        }"#;
        let error = parse_rustdesk_pro_import_json(json).expect_err("dangling book reference");
        assert!(
            matches!(error, ImportParseError::InvalidJson(message) if message.contains("unknown address-book reference"))
        );
    }

    #[test]
    fn dry_run_requires_explicit_scope_mapping_and_does_not_map_name() {
        let report = dry_run_import(&document(), &snapshot());
        assert!(report.blocked);
        assert_eq!(report.scopes[0].action, ReconciliationAction::Blocked);
        assert!(report.scopes[0]
            .reason
            .as_deref()
            .expect("reason")
            .contains("unproven"));
        assert_eq!(
            report.identities[0].action,
            ReconciliationAction::MatchedExisting
        );
        assert_eq!(
            report.devices[0].action,
            ReconciliationAction::MatchedExisting
        );
        assert_eq!(
            report.address_book_entries[0].action,
            ReconciliationAction::Mapped
        );
    }

    #[test]
    fn dry_run_rejects_duplicate_import_and_ambiguous_snapshot_ids() {
        let mut input = document();
        input.devices.push(input.devices[0].clone());
        let mut state = snapshot();
        state.devices.push(state.devices[0].clone());
        let report = dry_run_import(&input, &state);
        assert!(report.blocked);
        assert!(report
            .devices
            .iter()
            .all(|item| item.action == ReconciliationAction::Blocked));
    }

    #[test]
    fn dry_run_never_creates_unmatched_identity_without_credentials() {
        let mut input = document();
        input.users[0].username = "new-user".to_string();
        let report = dry_run_import(&input, &snapshot());
        assert_eq!(report.identities[0].action, ReconciliationAction::Blocked);
        assert!(report.identities[0]
            .reason
            .as_deref()
            .expect("reason")
            .contains("credential"));
    }
}
