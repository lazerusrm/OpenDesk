use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    pub device_uuid: Uuid,
    pub rustdesk_id: Option<String>,
    pub alias: String,
    pub hostname: Option<String>,
    pub os_family: Option<String>,
    pub os_version: Option<String>,
    pub architecture: Option<String>,
    pub rustdesk_version: Option<String>,
    pub site_uuid: Option<Uuid>,
    pub owner: Option<String>,
    pub notes: Option<String>,
    pub archived: bool,
    pub last_checkin_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceDraft {
    pub rustdesk_id: Option<String>,
    pub alias: String,
    pub hostname: Option<String>,
    pub os_family: Option<String>,
    pub os_version: Option<String>,
    pub architecture: Option<String>,
    pub rustdesk_version: Option<String>,
    pub site_uuid: Option<Uuid>,
    pub owner: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceSearchQuery {
    pub term: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeviceValidationError {
    #[error("alias must not be empty")]
    EmptyAlias,
    #[error("alias is too long")]
    AliasTooLong,
    #[error("rustdesk_id is too long")]
    RustdeskIdTooLong,
}

pub fn normalize_optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn normalize_device_draft(mut draft: DeviceDraft) -> DeviceDraft {
    draft.alias = draft.alias.trim().to_string();
    draft.rustdesk_id = normalize_optional_trimmed(draft.rustdesk_id);
    draft.hostname = normalize_optional_trimmed(draft.hostname);
    draft.owner = normalize_optional_trimmed(draft.owner);
    draft.os_family = normalize_optional_trimmed(draft.os_family);
    draft.os_version = normalize_optional_trimmed(draft.os_version);
    draft.architecture = normalize_optional_trimmed(draft.architecture);
    draft.rustdesk_version = normalize_optional_trimmed(draft.rustdesk_version);
    draft.notes = draft
        .notes
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    draft
}

pub fn validate_device_draft(draft: &DeviceDraft) -> Result<(), DeviceValidationError> {
    let alias = draft.alias.trim();
    if alias.is_empty() {
        return Err(DeviceValidationError::EmptyAlias);
    }
    if alias.len() > 128 {
        return Err(DeviceValidationError::AliasTooLong);
    }
    if let Some(rustdesk_id) = &draft.rustdesk_id {
        if rustdesk_id.trim().len() > 64 {
            return Err(DeviceValidationError::RustdeskIdTooLong);
        }
    }
    Ok(())
}

pub const NOTES_DISPLAY_MAX_LEN: usize = 64;

pub fn format_notes_display(notes: Option<&str>) -> String {
    let Some(notes) = notes.map(str::trim).filter(|value| !value.is_empty()) else {
        return "-".to_string();
    };
    if notes.chars().count() <= NOTES_DISPLAY_MAX_LEN {
        return notes.to_string();
    }
    let truncated: String = notes.chars().take(NOTES_DISPLAY_MAX_LEN).collect();
    format!("{truncated}...")
}

pub fn notes_list_title(notes: Option<&str>) -> String {
    notes.map(str::trim).filter(|value| !value.is_empty()).unwrap_or("").to_string()
}

pub fn device_matches_search(device: &Device, query: &DeviceSearchQuery) -> bool {
    let term = query.term.trim().to_ascii_lowercase();
    if term.is_empty() {
        return true;
    }
    let fields = [
        device.alias.as_str(),
        device.hostname.as_deref().unwrap_or(""),
        device.rustdesk_id.as_deref().unwrap_or(""),
        device.owner.as_deref().unwrap_or(""),
        device.notes.as_deref().unwrap_or(""),
        device.os_family.as_deref().unwrap_or(""),
    ];
    fields.iter().any(|field| field.to_ascii_lowercase().contains(&term))
}

pub fn device_in_default_list(
    device: &Device,
    query: &DeviceSearchQuery,
    site_name: Option<&str>,
    tag_names: &[&str],
) -> bool {
    !device.archived
        && device_matches_search_with_metadata(device, query, site_name, tag_names)
}

pub fn devices_for_default_list<'a>(
    devices: &'a [Device],
    query: &DeviceSearchQuery,
    site_names: &std::collections::HashMap<uuid::Uuid, String>,
    device_tag_names: &std::collections::HashMap<uuid::Uuid, Vec<String>>,
) -> Vec<&'a Device> {
    devices
        .iter()
        .filter(|device| {
            let site_name = device
                .site_uuid
                .and_then(|uuid| site_names.get(&uuid).map(String::as_str));
            let tag_names: Vec<&str> = device_tag_names
                .get(&device.device_uuid)
                .map(|names| names.iter().map(String::as_str).collect())
                .unwrap_or_default();
            device_in_default_list(device, query, site_name, &tag_names)
        })
        .collect()
}

pub fn device_matches_search_with_metadata(
    device: &Device,
    query: &DeviceSearchQuery,
    site_name: Option<&str>,
    tag_names: &[&str],
) -> bool {
    if device_matches_search(device, query) {
        return true;
    }
    let term = query.term.trim().to_ascii_lowercase();
    if term.is_empty() {
        return true;
    }
    if site_name
        .map(|name| name.to_ascii_lowercase().contains(&term))
        .unwrap_or(false)
    {
        return true;
    }
    tag_names
        .iter()
        .any(|name| name.to_ascii_lowercase().contains(&term))
}

pub fn device_matches_search_with_site_name(
    device: &Device,
    query: &DeviceSearchQuery,
    site_name: Option<&str>,
) -> bool {
    device_matches_search_with_metadata(device, query, site_name, &[])
}

pub fn archive_device(device: &Device) -> Device {
    let mut updated = device.clone();
    updated.archived = true;
    updated
}

pub fn unarchive_device(device: &Device) -> Device {
    let mut updated = device.clone();
    updated.archived = false;
    updated
}

/// Preserve enrollment/check-in metadata when the edit form only exposes operator fields.
pub fn merge_device_update(form: DeviceDraft, existing: &Device) -> DeviceDraft {
    DeviceDraft {
        alias: form.alias,
        rustdesk_id: form.rustdesk_id.or_else(|| existing.rustdesk_id.clone()),
        hostname: form.hostname.or_else(|| existing.hostname.clone()),
        owner: form.owner.or_else(|| existing.owner.clone()),
        notes: form.notes.or_else(|| existing.notes.clone()),
        os_family: existing.os_family.clone(),
        os_version: existing.os_version.clone(),
        architecture: existing.architecture.clone(),
        rustdesk_version: existing.rustdesk_version.clone(),
        // Site select always submits a value; empty means unassigned.
        site_uuid: form.site_uuid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> Device {
        Device {
            device_uuid: Uuid::new_v4(),
            rustdesk_id: Some("123456789".to_string()),
            alias: "Workstation".to_string(),
            hostname: Some("ws-01".to_string()),
            os_family: Some("linux".to_string()),
            os_version: None,
            architecture: None,
            rustdesk_version: None,
            site_uuid: None,
            owner: Some("ops".to_string()),
            notes: Some("lab device".to_string()),
            archived: false,
            last_checkin_at: None,
        }
    }

    #[test]
    fn validate_device_draft_requires_alias() {
        let draft = DeviceDraft {
            alias: "  ".to_string(),
            ..Default::default()
        };
        assert_eq!(
            validate_device_draft(&draft),
            Err(DeviceValidationError::EmptyAlias)
        );
    }

    #[test]
    fn devices_for_default_list_excludes_archived_devices() {
        let mut active = sample_device();
        active.alias = "Active Workstation".to_string();
        let mut archived = sample_device();
        archived.alias = "Archived Workstation".to_string();
        archived.archived = true;
        let devices = vec![active, archived];
        let query = DeviceSearchQuery::default();
        let site_names = std::collections::HashMap::new();
        let tag_names = std::collections::HashMap::new();
        let listed = devices_for_default_list(&devices, &query, &site_names, &tag_names);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].alias, "Active Workstation");
    }

    #[test]
    fn devices_for_default_list_hides_archived_even_when_search_matches() {
        let mut archived = sample_device();
        archived.alias = "Archived Workstation".to_string();
        archived.archived = true;
        let devices = vec![archived];
        let query = DeviceSearchQuery {
            term: "Archived".to_string(),
        };
        let site_names = std::collections::HashMap::new();
        let tag_names = std::collections::HashMap::new();
        let listed = devices_for_default_list(&devices, &query, &site_names, &tag_names);
        assert!(listed.is_empty());
    }

    #[test]
    fn device_matches_search_includes_site_name() {
        let device = sample_device();
        let query = DeviceSearchQuery {
            term: "lab floor".to_string(),
        };
        assert!(device_matches_search_with_site_name(
            &device,
            &query,
            Some("Main Lab Floor")
        ));
    }

    #[test]
    fn device_matches_search_includes_tag_name() {
        let device = sample_device();
        let query = DeviceSearchQuery {
            term: "production".to_string(),
        };
        assert!(device_matches_search_with_metadata(
            &device,
            &query,
            None,
            &["Production Fleet"]
        ));
    }

    #[test]
    fn format_notes_display_truncates_long_values() {
        let long = "a".repeat(80);
        let display = format_notes_display(Some(&long));
        assert!(display.ends_with("..."));
        assert!(display.chars().count() <= NOTES_DISPLAY_MAX_LEN + 3);
        assert_eq!(format_notes_display(None), "-");
        assert_eq!(format_notes_display(Some("  ")), "-");
        assert_eq!(format_notes_display(Some("lab device")), "lab device");
    }

    #[test]
    fn device_matches_search_by_notes() {
        let device = sample_device();
        let query = DeviceSearchQuery {
            term: "lab device".to_string(),
        };
        assert!(device_matches_search(&device, &query));
    }

    #[test]
    fn device_matches_search_by_rustdesk_id() {
        let device = sample_device();
        let query = DeviceSearchQuery {
            term: "123456".to_string(),
        };
        assert!(device_matches_search(&device, &query));
    }

    #[test]
    fn archive_device_sets_flag() {
        let device = sample_device();
        let archived = archive_device(&device);
        assert!(archived.archived);
    }

    #[test]
    fn normalize_device_draft_trims_hostname_and_rustdesk_id() {
        let draft = normalize_device_draft(DeviceDraft {
            alias: "  Workstation  ".to_string(),
            rustdesk_id: Some(" 123456789 ".to_string()),
            hostname: Some(" ws-01 ".to_string()),
            ..Default::default()
        });
        assert_eq!(draft.alias, "Workstation");
        assert_eq!(draft.rustdesk_id.as_deref(), Some("123456789"));
        assert_eq!(draft.hostname.as_deref(), Some("ws-01"));
    }

    #[test]
    fn merge_device_update_clears_site_when_form_unassigns() {
        let mut existing = sample_device();
        existing.site_uuid = Some(Uuid::new_v4());
        let form = DeviceDraft {
            alias: "Renamed".to_string(),
            site_uuid: None,
            ..Default::default()
        };
        let merged = merge_device_update(form, &existing);
        assert_eq!(merged.site_uuid, None);
        assert_eq!(merged.rustdesk_id.as_deref(), Some("123456789"));
    }

    #[test]
    fn merge_device_update_assigns_site_from_form() {
        let existing = sample_device();
        let new_site = Uuid::new_v4();
        let form = DeviceDraft {
            alias: "Renamed".to_string(),
            site_uuid: Some(new_site),
            ..Default::default()
        };
        let merged = merge_device_update(form, &existing);
        assert_eq!(merged.site_uuid, Some(new_site));
    }

    #[test]
    fn merge_device_update_preserves_enrollment_metadata() {
        let existing = sample_device();
        let form = DeviceDraft {
            alias: "Renamed".to_string(),
            rustdesk_id: None,
            hostname: None,
            owner: None,
            notes: Some("updated note".to_string()),
            ..Default::default()
        };
        let merged = merge_device_update(form, &existing);
        assert_eq!(merged.alias, "Renamed");
        assert_eq!(merged.os_family.as_deref(), Some("linux"));
        assert_eq!(merged.rustdesk_id.as_deref(), Some("123456789"));
        assert_eq!(merged.notes.as_deref(), Some("updated note"));
    }
}