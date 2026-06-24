use std::collections::HashMap;

use uuid::Uuid;

use super::device::Device;
use super::device_list::{devices_for_default_list, DeviceSearchQuery};

pub const DEVICE_CSV_HEADER: &str = "device_uuid,alias,rustdesk_id,hostname,site,tags,notes,owner,os_family,os_version,architecture,rustdesk_version,last_checkin_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceCsvRow {
    pub device_uuid: String,
    pub alias: String,
    pub rustdesk_id: String,
    pub hostname: String,
    pub site: String,
    pub tags: String,
    pub notes: String,
    pub owner: String,
    pub os_family: String,
    pub os_version: String,
    pub architecture: String,
    pub rustdesk_version: String,
    pub last_checkin_at: String,
}

pub fn escape_csv_field(value: &str) -> String {
    if value.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn optional_field(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

pub fn device_to_csv_row(
    device: &Device,
    site_name: Option<&str>,
    tag_names: &[String],
) -> DeviceCsvRow {
    DeviceCsvRow {
        device_uuid: device.device_uuid.to_string(),
        alias: device.alias.clone(),
        rustdesk_id: optional_field(device.rustdesk_id.as_deref()),
        hostname: optional_field(device.hostname.as_deref()),
        site: site_name.unwrap_or("").to_string(),
        tags: tag_names.join(", "),
        notes: optional_field(device.notes.as_deref()),
        owner: optional_field(device.owner.as_deref()),
        os_family: optional_field(device.os_family.as_deref()),
        os_version: optional_field(device.os_version.as_deref()),
        architecture: optional_field(device.architecture.as_deref()),
        rustdesk_version: optional_field(device.rustdesk_version.as_deref()),
        last_checkin_at: optional_field(device.last_checkin_at.as_deref()),
    }
}

pub fn render_csv_row(row: &DeviceCsvRow) -> String {
    [
        row.device_uuid.as_str(),
        row.alias.as_str(),
        row.rustdesk_id.as_str(),
        row.hostname.as_str(),
        row.site.as_str(),
        row.tags.as_str(),
        row.notes.as_str(),
        row.owner.as_str(),
        row.os_family.as_str(),
        row.os_version.as_str(),
        row.architecture.as_str(),
        row.rustdesk_version.as_str(),
        row.last_checkin_at.as_str(),
    ]
    .into_iter()
    .map(escape_csv_field)
    .collect::<Vec<_>>()
    .join(",")
}

pub fn render_devices_csv(
    devices: &[Device],
    query: &DeviceSearchQuery,
    site_names: &HashMap<Uuid, String>,
    device_tag_names: &HashMap<Uuid, Vec<String>>,
) -> String {
    let listed = devices_for_default_list(devices, query, site_names, device_tag_names);
    let mut lines = vec![DEVICE_CSV_HEADER.to_string()];
    for device in listed {
        let site_name = device
            .site_uuid
            .and_then(|uuid| site_names.get(&uuid).map(String::as_str));
        let tag_names = device_tag_names
            .get(&device.device_uuid)
            .cloned()
            .unwrap_or_default();
        let row = device_to_csv_row(device, site_name, &tag_names);
        lines.push(render_csv_row(&row));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::device::Device;

    fn sample_device() -> Device {
        Device {
            device_uuid: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid"),
            rustdesk_id: Some("123456789".to_string()),
            alias: "Lab Workstation".to_string(),
            hostname: Some("ws-01".to_string()),
            os_family: Some("linux".to_string()),
            os_version: Some("24.04".to_string()),
            architecture: Some("x86_64".to_string()),
            rustdesk_version: Some("1.4.8".to_string()),
            site_uuid: None,
            owner: Some("ops".to_string()),
            notes: Some("Needs reboot".to_string()),
            archived: false,
            last_checkin_at: Some("2026-06-24T12:00:00Z".to_string()),
        }
    }

    #[test]
    fn escape_csv_field_quotes_commas_and_newlines() {
        assert_eq!(escape_csv_field("plain"), "plain");
        assert_eq!(escape_csv_field("a,b"), "\"a,b\"");
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape_csv_field("line\nbreak"), "\"line\nbreak\"");
    }

    #[test]
    fn render_devices_csv_includes_header_and_device_fields() {
        let site_uuid = Uuid::new_v4();
        let mut device = sample_device();
        device.site_uuid = Some(site_uuid);
        let devices = vec![device];
        let site_names = HashMap::from([(site_uuid, "Main Lab".to_string())]);
        let tag_names = HashMap::from([(
            devices[0].device_uuid,
            vec!["Production".to_string()],
        )]);
        let csv = render_devices_csv(
            &devices,
            &DeviceSearchQuery::default(),
            &site_names,
            &tag_names,
        );
        assert!(csv.starts_with(DEVICE_CSV_HEADER));
        assert!(csv.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(csv.contains("Lab Workstation"));
        assert!(csv.contains("123456789"));
        assert!(csv.contains("Main Lab"));
        assert!(csv.contains("Production"));
        assert!(csv.contains("Needs reboot"));
    }

    #[test]
    fn render_devices_csv_excludes_archived_devices() {
        let mut archived = sample_device();
        archived.archived = true;
        archived.alias = "Archived Device".to_string();
        let active = sample_device();
        let devices = vec![archived, active];
        let csv = render_devices_csv(
            &devices,
            &DeviceSearchQuery::default(),
            &HashMap::new(),
            &HashMap::new(),
        );
        assert!(!csv.contains("Archived Device"));
        assert!(csv.contains("Lab Workstation"));
    }
}