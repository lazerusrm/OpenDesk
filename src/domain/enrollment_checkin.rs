use uuid::Uuid;

use crate::domain::device::Device;

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
        .or_else(|| {
            lookup
                .by_hostname
                .as_ref()
                .map(|device| device.device_uuid)
        })
}

pub fn hostname_lookup_key(hostname: Option<&str>) -> Option<String> {
    hostname
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
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
}