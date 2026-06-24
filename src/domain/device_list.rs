use std::collections::HashMap;

use uuid::Uuid;

use super::device::Device;

#[derive(Debug, Clone, Default)]
pub struct DeviceSearchQuery {
    pub term: String,
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

pub fn rustdesk_id_copy_text(rustdesk_id: Option<&str>) -> Option<String> {
    super::device::normalize_optional_trimmed(rustdesk_id.map(str::to_string))
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
    site_names: &HashMap<Uuid, String>,
    device_tag_names: &HashMap<Uuid, Vec<String>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::Device;
    use uuid::Uuid;

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
    fn devices_for_default_list_excludes_archived_devices() {
        let mut active = sample_device();
        active.alias = "Active Workstation".to_string();
        let mut archived = sample_device();
        archived.alias = "Archived Workstation".to_string();
        archived.archived = true;
        let devices = vec![active, archived];
        let query = DeviceSearchQuery::default();
        let site_names = HashMap::new();
        let tag_names = HashMap::new();
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
        let site_names = HashMap::new();
        let tag_names = HashMap::new();
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
    fn rustdesk_id_copy_text_returns_trimmed_id() {
        assert_eq!(
            rustdesk_id_copy_text(Some(" 123456789 ")),
            Some("123456789".to_string())
        );
        assert_eq!(rustdesk_id_copy_text(None), None);
        assert_eq!(rustdesk_id_copy_text(Some("   ")), None);
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
}