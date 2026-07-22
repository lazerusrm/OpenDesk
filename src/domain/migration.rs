//! Dry-run-only adapter for sanitized RustDesk Pro exports.
//!
//! This boundary accepts only the fields needed to reconcile identities, groups,
//! devices, and address-book entries. It never writes to storage and deliberately
//! has no credential, session, key, or secret fields.

use uuid::Uuid;

use super::{device::Device, site::Site, user::User};

#[path = "migration_export.rs"]
mod migration_export;
pub use migration_export::{
    parse_rustdesk_pro_import_json, ImportParseError, RustDeskProAddressBook,
    RustDeskProAddressBookEntry, RustDeskProCrossGroupEdge, RustDeskProDevice, RustDeskProGroup,
    RustDeskProImportDocument, RustDeskProUser, RUSTDESK_PRO_IMPORT_SCHEMA_VERSION,
};

#[path = "migration_manifest.rs"]
mod migration_manifest;
pub use migration_manifest::{
    expected_import_counts, parse_approved_migration_manifest_json, planned_import_target,
    sha256_digest, validate_manifest_preconditions, ApprovedMigrationManifest, CredentialPath,
    IdentityCollisionDisposition, ManifestAction, ManifestExpectedCounts, ManifestParseError,
    ManifestPreconditionError, ManifestSemanticError, ManifestSourceCoverageError,
    ManifestSourceKind, ManifestValidationError, MigrationDisposition, RolePath, VisibilityIntent,
    APPROVED_MIGRATION_MANIFEST_SCHEMA_VERSION,
};

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
            cross_group_edges: vec![],
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
