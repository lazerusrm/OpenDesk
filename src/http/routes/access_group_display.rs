use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct AccessGroupCreateForm {
    pub csrf_token: String,
    pub name: String,
}

#[derive(Deserialize)]
pub(super) struct UuidReplacementForm {
    pub csrf_token: String,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    pub user_uuid: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    pub device_uuid: Vec<String>,
}

#[derive(Deserialize)]
pub(super) struct AccessGroupAccessForm {
    pub csrf_token: String,
    #[serde(default, deserialize_with = "deserialize_form_string_vec")]
    pub outgoing_access_group_uuid: Vec<String>,
}

pub(super) fn parse_uuids(values: &[String]) -> Option<Vec<Uuid>> {
    values
        .iter()
        .map(|value| Uuid::parse_str(value.trim()))
        .collect::<Result<_, _>>()
        .ok()
}

pub(super) fn deserialize_form_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
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

pub(super) fn rustdesk_id_text(rustdesk_id: &Option<String>) -> String {
    rustdesk_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

pub(super) fn device_label(alias: &str, rustdesk_id: &str) -> String {
    if rustdesk_id.is_empty() {
        alias.to_string()
    } else {
        format!("{alias} · {rustdesk_id}")
    }
}

pub(super) fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

pub(super) fn join_or_empty(names: &[String], empty: &str) -> String {
    if names.is_empty() {
        empty.to_string()
    } else {
        names.join(", ")
    }
}
