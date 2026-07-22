use super::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum ReconciliationAction {
    MatchedExisting,
    WouldCreate,
    Mapped,
    Blocked,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReconciliationItem {
    pub external_id: String,
    pub source_parent_id: Option<String>,
    pub action: ReconciliationAction,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ImportDryRunReport {
    pub schema_version: u32,
    pub identities: Vec<ReconciliationItem>,
    pub scopes: Vec<ReconciliationItem>,
    pub devices: Vec<ReconciliationItem>,
    pub address_books: Vec<ReconciliationItem>,
    pub address_book_entries: Vec<ReconciliationItem>,
    pub cross_group_edges: Vec<ReconciliationItem>,
    pub blocked: bool,
}

impl ImportDryRunReport {
    fn new(document: &RustDeskProImportDocument) -> Self {
        Self {
            schema_version: document.schema_version,
            identities: Vec::new(),
            scopes: Vec::new(),
            devices: Vec::new(),
            address_books: Vec::new(),
            address_book_entries: Vec::new(),
            cross_group_edges: Vec::new(),
            blocked: false,
        }
    }

    fn push_blocked(&mut self, target: ReconciliationTarget, id: String, reason: &str) {
        self.push_blocked_with_parent(target, id, None, reason);
    }

    fn push_blocked_with_parent(
        &mut self,
        target: ReconciliationTarget,
        id: String,
        source_parent_id: Option<String>,
        reason: &str,
    ) {
        let item = ReconciliationItem {
            external_id: id,
            source_parent_id,
            action: ReconciliationAction::Blocked,
            reason: Some(reason.to_string()),
        };
        match target {
            ReconciliationTarget::Identities => self.identities.push(item),
            ReconciliationTarget::Scopes => self.scopes.push(item),
            ReconciliationTarget::Devices => self.devices.push(item),
            ReconciliationTarget::AddressBooks => self.address_books.push(item),
            ReconciliationTarget::AddressBookEntries => self.address_book_entries.push(item),
        }
        self.blocked = true;
    }
}

#[derive(Debug, Clone, Copy)]
enum ReconciliationTarget {
    Identities,
    Scopes,
    Devices,
    AddressBooks,
    AddressBookEntries,
}

/// Reconcile an import against a caller-provided snapshot without changing it.
/// This function is intentionally the only operation exposed by this module;
/// persistence and credential provisioning remain separate owner decisions.
pub fn dry_run_import(
    document: &RustDeskProImportDocument,
    snapshot: &MigrationSnapshot,
) -> ImportDryRunReport {
    let mut report = ImportDryRunReport::new(document);
    reconcile_identities(document, snapshot, &mut report);
    reconcile_scopes(document, snapshot, &mut report);
    reconcile_devices(document, snapshot, &mut report);
    reconcile_address_books(document, &mut report);
    reconcile_address_book_entries(document, snapshot, &mut report);
    reconcile_cross_group_edges(document, &mut report);
    report
}

fn reconcile_identities(
    document: &RustDeskProImportDocument,
    snapshot: &MigrationSnapshot,
    report: &mut ImportDryRunReport,
) {
    let mut imported_ids = HashSet::new();
    let mut imported_names = HashSet::new();
    let mut snapshot_names: HashMap<String, usize> = HashMap::new();
    for user in &snapshot.users {
        *snapshot_names
            .entry(user.username.trim().to_ascii_lowercase())
            .or_default() += 1;
    }
    for user in &document.users {
        let name_key = user.username.trim().to_ascii_lowercase();
        if user.user_id.trim().is_empty()
            || !imported_ids.insert(user.user_id.clone())
            || !imported_names.insert(name_key.clone())
        {
            report.push_blocked(
                ReconciliationTarget::Identities,
                user.user_id.clone(),
                "duplicate or empty external identity",
            );
            continue;
        }
        match snapshot_names.get(&name_key).copied() {
            Some(1) => report.identities.push(ReconciliationItem {
                external_id: user.user_id.clone(),
                source_parent_id: None,
                action: ReconciliationAction::MatchedExisting,
                reason: Some(
                    "matched by username; credentials and role are not imported".to_string(),
                ),
            }),
            Some(_) => report.push_blocked(
                ReconciliationTarget::Identities,
                user.user_id.clone(),
                "username is ambiguous in the OpenDesk snapshot",
            ),
            None => report.push_blocked(
                ReconciliationTarget::Identities,
                user.user_id.clone(),
                "no existing identity; credential provisioning is outside this dry run",
            ),
        }
    }
}

fn reconcile_scopes(
    document: &RustDeskProImportDocument,
    snapshot: &MigrationSnapshot,
    report: &mut ImportDryRunReport,
) {
    let site_ids: HashSet<Uuid> = snapshot.sites.iter().map(|site| site.site_uuid).collect();
    let mut mappings: HashMap<&str, Option<Uuid>> = HashMap::new();
    for mapping in &snapshot.scope_site_mappings {
        let entry = mappings
            .entry(mapping.group_id.as_str())
            .or_insert(Some(mapping.site_uuid));
        if entry != &Some(mapping.site_uuid) {
            *entry = None;
        }
    }
    let mut imported_ids = HashSet::new();
    for group in &document.groups {
        if group.group_id.trim().is_empty() || !imported_ids.insert(group.group_id.clone()) {
            report.push_blocked(
                ReconciliationTarget::Scopes,
                group.group_id.clone(),
                "duplicate or empty external scope",
            );
            continue;
        }
        match mappings.get(group.group_id.as_str()) {
            Some(Some(site_uuid)) if site_ids.contains(site_uuid) => {
                report.scopes.push(ReconciliationItem {
                    external_id: group.group_id.clone(),
                    source_parent_id: None,
                    action: ReconciliationAction::Mapped,
                    reason: Some(format!("explicit operator mapping to site {site_uuid}")),
                })
            }
            Some(Some(_)) => report.push_blocked(
                ReconciliationTarget::Scopes,
                group.group_id.clone(),
                "scope mapping references a missing site",
            ),
            Some(None) => report.push_blocked(
                ReconciliationTarget::Scopes,
                group.group_id.clone(),
                "conflicting scope mappings",
            ),
            None => report.push_blocked(
                ReconciliationTarget::Scopes,
                group.group_id.clone(),
                "scope semantics are unproven; provide explicit mapping evidence",
            ),
        }
    }
}

fn reconcile_devices(
    document: &RustDeskProImportDocument,
    snapshot: &MigrationSnapshot,
    report: &mut ImportDryRunReport,
) {
    let mut snapshot_ids: HashMap<&str, usize> = HashMap::new();
    for device in &snapshot.devices {
        if let Some(rustdesk_id) = device.rustdesk_id.as_deref() {
            *snapshot_ids.entry(rustdesk_id).or_default() += 1;
        }
    }
    let mut imported_ids = HashSet::new();
    for device in &document.devices {
        if device.rustdesk_id.trim().is_empty() || !imported_ids.insert(device.rustdesk_id.clone())
        {
            report.push_blocked(
                ReconciliationTarget::Devices,
                device.rustdesk_id.clone(),
                "duplicate or empty rustdesk_id in import",
            );
            continue;
        }
        match snapshot_ids.get(device.rustdesk_id.as_str()).copied() {
            Some(1) => report.devices.push(ReconciliationItem {
                external_id: device.rustdesk_id.clone(),
                source_parent_id: None,
                action: ReconciliationAction::MatchedExisting,
                reason: Some("matched by rustdesk_id".to_string()),
            }),
            Some(_) => report.push_blocked(
                ReconciliationTarget::Devices,
                device.rustdesk_id.clone(),
                "rustdesk_id is ambiguous in the OpenDesk snapshot",
            ),
            None => report.devices.push(ReconciliationItem {
                external_id: device.rustdesk_id.clone(),
                source_parent_id: None,
                action: ReconciliationAction::WouldCreate,
                reason: Some("device has no matching rustdesk_id".to_string()),
            }),
        }
    }
}

fn reconcile_address_books(document: &RustDeskProImportDocument, report: &mut ImportDryRunReport) {
    let mut imported_ids = HashSet::new();
    for book in &document.address_books {
        if book.address_book_id.trim().is_empty()
            || !imported_ids.insert(book.address_book_id.clone())
        {
            report.push_blocked(
                ReconciliationTarget::AddressBooks,
                book.address_book_id.clone(),
                "duplicate or empty external address-book identity",
            );
        } else {
            report.address_books.push(ReconciliationItem {
                external_id: book.address_book_id.clone(),
                source_parent_id: None,
                action: ReconciliationAction::WouldCreate,
                reason: Some(
                    "address-book container is report-only; no native OpenDesk entity is written"
                        .to_string(),
                ),
            });
        }
    }
}

fn reconcile_address_book_entries(
    document: &RustDeskProImportDocument,
    snapshot: &MigrationSnapshot,
    report: &mut ImportDryRunReport,
) {
    let books: HashSet<&str> = document
        .address_books
        .iter()
        .map(|book| book.address_book_id.as_str())
        .collect();
    let devices: HashMap<&str, usize> = snapshot
        .devices
        .iter()
        .filter_map(|device| device.rustdesk_id.as_deref())
        .fold(HashMap::new(), |mut ids, rustdesk_id| {
            *ids.entry(rustdesk_id).or_default() += 1;
            ids
        });
    let mut imported_keys = HashSet::new();
    for entry in &document.address_book_entries {
        let key = (entry.address_book_id.clone(), entry.rustdesk_id.clone());
        if !imported_keys.insert(key.clone()) {
            report.push_blocked_with_parent(
                ReconciliationTarget::AddressBookEntries,
                entry.rustdesk_id.clone(),
                Some(entry.address_book_id.clone()),
                "duplicate address-book entry",
            );
            continue;
        }
        if !books.contains(entry.address_book_id.as_str()) {
            report.push_blocked_with_parent(
                ReconciliationTarget::AddressBookEntries,
                entry.rustdesk_id.clone(),
                Some(entry.address_book_id.clone()),
                "address book does not exist in import",
            );
            continue;
        }
        match devices.get(entry.rustdesk_id.as_str()).copied() {
            Some(1) => report.address_book_entries.push(ReconciliationItem {
                external_id: entry.rustdesk_id.clone(),
                source_parent_id: Some(entry.address_book_id.clone()),
                action: ReconciliationAction::Mapped,
                reason: Some(
                    "mapped to existing device by rustdesk_id; secrets excluded".to_string(),
                ),
            }),
            Some(_) => report.push_blocked_with_parent(
                ReconciliationTarget::AddressBookEntries,
                entry.rustdesk_id.clone(),
                Some(entry.address_book_id.clone()),
                "rustdesk_id is ambiguous in the OpenDesk snapshot",
            ),
            None => report.push_blocked_with_parent(
                ReconciliationTarget::AddressBookEntries,
                entry.rustdesk_id.clone(),
                Some(entry.address_book_id.clone()),
                "no matching device; address-book entry is not imported",
            ),
        }
    }
}

fn reconcile_cross_group_edges(
    document: &RustDeskProImportDocument,
    report: &mut ImportDryRunReport,
) {
    for edge in &document.cross_group_edges {
        report.cross_group_edges.push(ReconciliationItem {
            external_id: edge.edge_id.clone(),
            source_parent_id: None,
            action: ReconciliationAction::WouldCreate,
            reason: Some(
                "cross-group edge is report-only; no native entity is written".to_string(),
            ),
        });
    }
}
