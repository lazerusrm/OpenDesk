use std::collections::HashSet;

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::migration_contract::{
    AddressBookPrincipalType, MigrationContractError, MigrationRunState, SanitizedMigrationExport,
    SourceProvenance, SANITIZED_MIGRATION_SCHEMA_VERSION,
};

pub fn validate_export(document: &SanitizedMigrationExport) -> Result<(), MigrationContractError> {
    if document.schema_version != SANITIZED_MIGRATION_SCHEMA_VERSION {
        return Err(MigrationContractError::UnsupportedSchemaVersion);
    }
    validate_provenance(&document.provenance)?;
    validate_run(&document.run)?;
    if document.provenance.snapshot_sha256 != document.run.source_snapshot_sha256 {
        return Err(MigrationContractError::InvalidSnapshotDigest);
    }
    let mut unsupported_categories = HashSet::new();
    for item in &document.unsupported_semantics {
        if item.count == 0 || !unsupported_categories.insert(item.category) {
            return Err(MigrationContractError::InvalidSourceValue {
                field: "unsupported_semantics",
            });
        }
    }
    let mut users = HashSet::new();
    for item in &document.users {
        validate_text(&item.source_user_id, "source_user_id")?;
        validate_text(&item.username, "username")?;
        if !users.insert(item.source_user_id.as_str()) {
            return Err(MigrationContractError::DuplicateSourceIdentifier {
                field: "source_user_id",
            });
        }
        if !item.credential_reset_required {
            return Err(MigrationContractError::CredentialResetRequired);
        }
    }
    let mut groups = HashSet::new();
    for item in &document.groups {
        validate_text(&item.source_group_id, "source_group_id")?;
        validate_text(&item.name, "group_name")?;
        if !groups.insert(item.source_group_id.as_str()) {
            return Err(MigrationContractError::DuplicateSourceIdentifier {
                field: "source_group_id",
            });
        }
    }
    for item in &document.user_group_memberships {
        if !users.contains(item.source_user_id.as_str())
            || !groups.contains(item.source_group_id.as_str())
        {
            return Err(MigrationContractError::UnknownMembershipReference);
        }
    }
    let mut cross_group_access = HashSet::new();
    for item in &document.cross_group_access {
        if !groups.contains(item.source_group_id.as_str())
            || !groups.contains(item.target_group_id.as_str())
        {
            return Err(MigrationContractError::UnknownCrossGroupReference);
        }
        if item.source_group_id == item.target_group_id
            || !cross_group_access
                .insert((item.source_group_id.as_str(), item.target_group_id.as_str()))
        {
            return Err(MigrationContractError::DuplicateCrossGroupAccess);
        }
    }
    let mut devices = HashSet::new();
    for item in &document.devices {
        validate_text(&item.rustdesk_id, "rustdesk_id")?;
        validate_text(&item.alias, "device_alias")?;
        if !devices.insert(item.rustdesk_id.as_str()) {
            return Err(MigrationContractError::DuplicateSourceIdentifier {
                field: "rustdesk_id",
            });
        }
        if item
            .owner_source_user_id
            .as_deref()
            .is_some_and(|owner| !users.contains(owner))
        {
            return Err(MigrationContractError::UnknownDeviceOwner);
        }
        for group_id in &item.source_group_ids {
            if !groups.contains(group_id.as_str()) {
                return Err(MigrationContractError::UnknownDeviceGroup);
            }
        }
    }
    let mut books = HashSet::new();
    for book in &document.address_books {
        validate_text(&book.source_address_book_id, "source_address_book_id")?;
        validate_text(&book.name, "address_book_name")?;
        if !books.insert(book.source_address_book_id.as_str()) {
            return Err(MigrationContractError::DuplicateSourceIdentifier {
                field: "source_address_book_id",
            });
        }
        let owner = book
            .owner_source_user_id
            .as_deref()
            .ok_or(MigrationContractError::UnknownAddressBookOwner)?;
        validate_text(owner, "owner_source_user_id")?;
        if !users.contains(owner) {
            return Err(MigrationContractError::UnknownAddressBookOwner);
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
    let mut entries = HashSet::new();
    for entry in &document.address_book_entries {
        validate_text(
            &entry.source_address_book_id,
            "entry_source_address_book_id",
        )?;
        validate_text(&entry.rustdesk_id, "entry_rustdesk_id")?;
        validate_text(&entry.alias, "address_book_entry_alias")?;
        if !entries.insert((
            entry.source_address_book_id.as_str(),
            entry.rustdesk_id.as_str(),
        )) {
            return Err(MigrationContractError::DuplicateSourceIdentifier {
                field: "address_book_entry",
            });
        }
        if !books.contains(entry.source_address_book_id.as_str()) {
            return Err(MigrationContractError::UnknownAddressBook);
        }
        if !devices.contains(entry.rustdesk_id.as_str()) {
            return Err(MigrationContractError::UnknownAddressBookDevice);
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

pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), MigrationContractError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(MigrationContractError::InvalidSourceValue { field });
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<(), MigrationContractError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(MigrationContractError::InvalidSnapshotDigest);
    }
    Ok(())
}

pub(super) fn validate_timestamp(
    value: &str,
    field: &'static str,
) -> Result<(), MigrationContractError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| MigrationContractError::InvalidTimestamp { field })?;
    if parsed.format(&Rfc3339).ok().as_deref() != Some(value) {
        return Err(MigrationContractError::InvalidTimestamp { field });
    }
    Ok(())
}

pub(super) fn find_sensitive_field(value: &serde_json::Value, path: &str) -> Option<String> {
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
