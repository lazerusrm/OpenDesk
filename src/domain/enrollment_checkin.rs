use uuid::Uuid;

use crate::domain::device::{Device, DeviceDraft};

/// Lookup results for duplicate detection during endpoint enrollment check-in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrollmentDeviceLookup {
    pub by_rustdesk_id: Option<Device>,
    pub by_hostname: Option<Device>,
}

/// Select an existing device to update instead of creating a duplicate.
/// RustDesk ID takes precedence over hostname per validation case E-003.
pub fn select_existing_device_for_checkin(lookup: &EnrollmentDeviceLookup) -> Option<Uuid> {
    lookup
        .by_rustdesk_id
        .as_ref()
        .map(|device| device.device_uuid)
        .or_else(|| lookup.by_hostname.as_ref().map(|device| device.device_uuid))
}

pub fn hostname_lookup_key(hostname: Option<&str>) -> Option<String> {
    hostname
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

/// Overlay official-client CLI `device_name` / `note` onto an existing device.
/// Blank values leave the stored alias and notes unchanged.
pub fn apply_cli_alias_notes(
    existing: &Device,
    device_name: Option<&str>,
    note: Option<&str>,
) -> DeviceDraft {
    let alias = device_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| existing.alias.clone());
    let notes = note
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .or_else(|| existing.notes.clone());
    DeviceDraft {
        rustdesk_id: existing.rustdesk_id.clone(),
        alias,
        hostname: existing.hostname.clone(),
        os_family: existing.os_family.clone(),
        os_version: existing.os_version.clone(),
        architecture: existing.architecture.clone(),
        rustdesk_version: existing.rustdesk_version.clone(),
        site_uuid: existing.site_uuid,
        owner: existing.owner.clone(),
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::Device;

    fn sample_device(alias: &str, rustdesk_id: &str, hostname: &str) -> Device {
        Device {
            device_uuid: Uuid::new_v4(),
            rustdesk_id: Some(rustdesk_id.to_string()),
            alias: alias.to_string(),
            hostname: Some(hostname.to_string()),
            os_family: None,
            os_version: None,
            architecture: None,
            rustdesk_version: None,
            site_uuid: None,
            owner: None,
            notes: None,
            archived: false,
            last_checkin_at: None,
        }
    }

    #[test]
    fn select_existing_device_prefers_rustdesk_id_match() {
        let by_rustdesk_id = sample_device("alpha", "111222333", "host-a");
        let by_hostname = sample_device("beta", "999888777", "host-b");
        let selected = select_existing_device_for_checkin(&EnrollmentDeviceLookup {
            by_rustdesk_id: Some(by_rustdesk_id.clone()),
            by_hostname: Some(by_hostname),
        });
        assert_eq!(selected, Some(by_rustdesk_id.device_uuid));
    }

    #[test]
    fn select_existing_device_falls_back_to_hostname_match() {
        let by_hostname = sample_device("beta", "999888777", "host-b");
        let selected = select_existing_device_for_checkin(&EnrollmentDeviceLookup {
            by_rustdesk_id: None,
            by_hostname: Some(by_hostname.clone()),
        });
        assert_eq!(selected, Some(by_hostname.device_uuid));
    }

    #[test]
    fn select_existing_device_returns_none_when_no_match() {
        assert_eq!(
            select_existing_device_for_checkin(&EnrollmentDeviceLookup::default()),
            None
        );
    }

    #[test]
    fn hostname_lookup_key_rejects_blank_values() {
        assert_eq!(hostname_lookup_key(Some("  ")), None);
        assert_eq!(
            hostname_lookup_key(Some("ws-01")),
            Some("ws-01".to_string())
        );
    }

    #[test]
    fn apply_cli_alias_notes_overlays_nonempty_fields() {
        let mut existing = sample_device("alpha", "111222333", "host-a");
        existing.notes = Some("kept".to_string());
        let draft = apply_cli_alias_notes(&existing, Some("  workstation-01  "), None);
        assert_eq!(draft.alias, "workstation-01");
        assert_eq!(draft.notes.as_deref(), Some("kept"));
        assert_eq!(draft.rustdesk_id.as_deref(), Some("111222333"));
        let cleared_name = apply_cli_alias_notes(&existing, Some("  "), Some("lab note"));
        assert_eq!(cleared_name.alias, "alpha");
        assert_eq!(cleared_name.notes.as_deref(), Some("lab note"));
    }
}
