use serde::Deserialize;
use uuid::Uuid;

use crate::domain::device::DeviceDraft;

fn deserialize_form_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        One(String),
        Many(Vec<String>),
    }

    Ok(match StringOrVec::deserialize(deserializer)? {
        StringOrVec::One(value) => vec![value],
        StringOrVec::Many(values) => values,
    })
}

#[derive(Deserialize)]
pub(crate) struct DeviceForm {
    #[serde(default)]
    pub(crate) csrf_token: String,
    pub(crate) alias: String,
    pub(crate) rustdesk_id: Option<String>,
    pub(crate) hostname: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) site_uuid: Option<String>,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    pub(crate) tag_uuids: Vec<String>,
}

pub(crate) fn parse_tag_uuids_from_form(tag_uuids: &[String]) -> Vec<Uuid> {
    tag_uuids
        .iter()
        .filter_map(|value| Uuid::parse_str(value.trim()).ok())
        .collect()
}

pub(crate) fn device_form_to_draft(form: DeviceForm) -> DeviceDraft {
    DeviceDraft {
        rustdesk_id: form.rustdesk_id.filter(|value| !value.trim().is_empty()),
        alias: form.alias,
        hostname: form.hostname.filter(|value| !value.trim().is_empty()),
        owner: form.owner.filter(|value| !value.trim().is_empty()),
        notes: form.notes.filter(|value| !value.trim().is_empty()),
        site_uuid: form
            .site_uuid
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| Uuid::parse_str(value.trim()).ok()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::DeviceForm;

    #[test]
    fn deserializes_single_tag_uuid_field() {
        let body = format!(
            "alias=Tagged+Workstation&tag_uuids={}",
            uuid::Uuid::new_v4()
        );
        let form: DeviceForm = serde_urlencoded::from_str(&body).expect("deserialize form");
        assert_eq!(form.tag_uuids.len(), 1);
    }
}
