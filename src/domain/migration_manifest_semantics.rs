//! Snapshot and dry-run semantic validation for approved manifests.

use std::collections::HashSet;
use thiserror::Error;
use uuid::Uuid;

use super::{ApprovedMigrationManifest, ManifestAction, ManifestSourceKind, MigrationDisposition};

const PLANNED_TARGET_NAMESPACE: Uuid = Uuid::from_u128(0x8c7c2d32_4b2d_5e86_9f0a_bbb89b3f0a10);

pub fn planned_import_target(disposition: &MigrationDisposition) -> Uuid {
    let parent = disposition.source_parent_id.as_deref().unwrap_or("");
    let identity = format!(
        "{:?}\0{}\0{}",
        disposition.source_kind, parent, disposition.source_id
    );
    Uuid::new_v5(&PLANNED_TARGET_NAMESPACE, identity.as_bytes())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestSemanticError {
    #[error("manifest target UUID does not exist in the current snapshot")]
    UnknownTarget,
    #[error("manifest target UUID is not valid for the source disposition")]
    InvalidTarget,
    #[error("manifest disposition action contradicts dry-run report")]
    ReportContradiction,
}

impl ApprovedMigrationManifest {
    pub fn validate_against_snapshot_and_report(
        &self,
        snapshot: &super::super::MigrationSnapshot,
        report: &super::super::ImportDryRunReport,
    ) -> Result<(), ManifestSemanticError> {
        let valid_targets: HashSet<Uuid> = snapshot
            .users
            .iter()
            .map(|item| item.user_uuid)
            .chain(snapshot.sites.iter().map(|item| item.site_uuid))
            .chain(snapshot.devices.iter().map(|item| item.device_uuid))
            .collect();
        for disposition in &self.dispositions {
            if let Some(target) = disposition.target_uuid {
                if matches!(disposition.action, ManifestAction::Import) {
                    if target != planned_import_target(disposition)
                        || valid_targets.contains(&target)
                    {
                        return Err(ManifestSemanticError::InvalidTarget);
                    }
                } else {
                    if !valid_targets.contains(&target) {
                        return Err(ManifestSemanticError::UnknownTarget);
                    }
                    if !target_kind_matches(disposition, target, snapshot) {
                        return Err(ManifestSemanticError::InvalidTarget);
                    }
                }
            } else if matches!(disposition.action, ManifestAction::Import) {
                return Err(ManifestSemanticError::InvalidTarget);
            }
            let expected = report_item_for_source(report, disposition);
            if !action_agrees_with_report(disposition, expected) {
                return Err(ManifestSemanticError::ReportContradiction);
            }
        }
        Ok(())
    }
}

fn target_kind_matches(
    disposition: &MigrationDisposition,
    target: Uuid,
    snapshot: &super::super::MigrationSnapshot,
) -> bool {
    match disposition.source_kind {
        ManifestSourceKind::User => {
            matches!(
                disposition.action,
                ManifestAction::Merge | ManifestAction::Retire
            ) && snapshot.users.iter().any(|item| item.user_uuid == target)
        }
        ManifestSourceKind::Group => {
            matches!(
                disposition.action,
                ManifestAction::Map | ManifestAction::Merge
            ) && snapshot.sites.iter().any(|item| item.site_uuid == target)
        }
        ManifestSourceKind::Device | ManifestSourceKind::AddressBookEntry => {
            matches!(
                disposition.action,
                ManifestAction::Merge | ManifestAction::Map
            ) && snapshot
                .devices
                .iter()
                .any(|item| item.device_uuid == target)
        }
        ManifestSourceKind::AddressBook | ManifestSourceKind::CrossGroupEdge => false,
    }
}

fn report_item_for_source<'a>(
    report: &'a super::super::ImportDryRunReport,
    disposition: &MigrationDisposition,
) -> Option<&'a super::super::ReconciliationItem> {
    let items = match disposition.source_kind {
        ManifestSourceKind::User => &report.identities,
        ManifestSourceKind::Group => &report.scopes,
        ManifestSourceKind::Device => &report.devices,
        ManifestSourceKind::AddressBook => &report.address_books,
        ManifestSourceKind::AddressBookEntry => &report.address_book_entries,
        ManifestSourceKind::CrossGroupEdge => &report.cross_group_edges,
    };
    items.iter().find(|item| {
        item.external_id == disposition.source_id
            && item.source_parent_id == disposition.source_parent_id
    })
}

fn action_agrees_with_report(
    disposition: &MigrationDisposition,
    item: Option<&super::super::ReconciliationItem>,
) -> bool {
    match (disposition.action, item.map(|item| item.action)) {
        (ManifestAction::Defer, Some(super::super::ReconciliationAction::Blocked)) => true,
        (ManifestAction::Retire, Some(super::super::ReconciliationAction::MatchedExisting)) => true,
        (ManifestAction::Map, Some(super::super::ReconciliationAction::Mapped)) => true,
        (ManifestAction::Merge, Some(super::super::ReconciliationAction::MatchedExisting)) => true,
        (ManifestAction::Import, Some(super::super::ReconciliationAction::WouldCreate)) => true,
        _ => false,
    }
}
