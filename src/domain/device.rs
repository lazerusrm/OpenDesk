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
        site_uuid: existing.site_uuid,
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