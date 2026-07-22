//! Sanitized external export boundary for migration review.
//!
//! This module owns the versioned allowlist, recursive sensitive-field guard,
//! and external-reference validation. It never writes imported data.

use serde::Deserialize;
use std::collections::HashSet;
use thiserror::Error;

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
    #[serde(default)]
    pub cross_group_edges: Vec<RustDeskProCrossGroupEdge>,
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RustDeskProCrossGroupEdge {
    pub edge_id: String,
    pub source_group_id: String,
    pub target_group_id: String,
}

/// `cross_group_edges` is an explicit sanitized source collection; it is never
/// inferred from device or address-book group memberships.

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportParseError {
    #[error("invalid import JSON: {0}")]
    InvalidJson(String),
    #[error("sensitive field rejected at {path}")]
    SensitiveField { path: String },
    #[error("unsupported import schema version")]
    UnsupportedSchemaVersion,
}

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
    validate_unique_source_ids(document)?;
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
    let mut edges = HashSet::new();
    for (index, edge) in document.cross_group_edges.iter().enumerate() {
        if !groups.contains(edge.source_group_id.as_str())
            || !groups.contains(edge.target_group_id.as_str())
        {
            return Err(ImportParseError::InvalidJson(format!(
                "unknown group reference at $.cross_group_edges[{index}]"
            )));
        }
        if edge.source_group_id == edge.target_group_id
            || !edges.insert((&edge.source_group_id, &edge.target_group_id))
        {
            return Err(ImportParseError::InvalidJson(format!(
                "duplicate or self cross-group edge at $.cross_group_edges[{index}]"
            )));
        }
    }
    Ok(())
}

fn validate_unique_source_ids(
    document: &RustDeskProImportDocument,
) -> Result<(), ImportParseError> {
    let mut ids = HashSet::new();
    for id in document
        .users
        .iter()
        .map(|item| ("user", item.user_id.as_str()))
        .chain(
            document
                .groups
                .iter()
                .map(|item| ("group", item.group_id.as_str())),
        )
        .chain(
            document
                .devices
                .iter()
                .map(|item| ("device", item.rustdesk_id.as_str())),
        )
        .chain(
            document
                .address_books
                .iter()
                .map(|item| ("address_book", item.address_book_id.as_str())),
        )
        .chain(
            document
                .cross_group_edges
                .iter()
                .map(|item| ("cross_group_edge", item.edge_id.as_str())),
        )
    {
        if id.1.trim().is_empty() || !ids.insert(id) {
            return Err(ImportParseError::InvalidJson(
                "duplicate or empty source identifier".to_string(),
            ));
        }
    }
    let mut entries = HashSet::new();
    for entry in &document.address_book_entries {
        if entry.address_book_id.trim().is_empty()
            || entry.rustdesk_id.trim().is_empty()
            || !entries.insert((&entry.address_book_id, &entry.rustdesk_id))
        {
            return Err(ImportParseError::InvalidJson(
                "duplicate or empty address-book entry".to_string(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn find_sensitive_field(value: &serde_json::Value, path: String) -> Option<String> {
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
    if name == "credential_path" {
        return false;
    }
    let compact = name.replace(['_', '-'], "");
    name.contains("password")
        || name.contains("credential")
        || compact.contains("privatekey")
        || name.contains("jwt")
        || name.contains("session")
        || name.contains("audit")
        || name.contains("topology")
        || name == "access_token"
        || name == "refresh_token"
        || name == "token"
        || name == "hash"
        || name.ends_with("_token")
        || name.contains("hash")
        || name == "secret"
        || name.contains("secret")
        || name.contains("key")
        || name == "api_key"
        || name.ends_with("_secret")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
