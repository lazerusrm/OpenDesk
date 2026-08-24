use serde::Deserialize;
use uuid::Uuid;

use crate::domain::address_book::normalize_optional_notes;

#[derive(Deserialize)]
pub(super) struct AddressBookCreateForm {
    pub csrf_token: String,
    pub name: String,
    pub book_kind: String,
}

#[derive(Deserialize)]
pub(super) struct AddressBookEntryForm {
    #[serde(default)]
    pub csrf_token: String,
    pub device_uuid: String,
    pub alias: String,
    pub notes: Option<String>,
    pub position: String,
}

pub(super) fn parse_entry(
    form: &AddressBookEntryForm,
) -> Option<(Uuid, String, Option<String>, u32)> {
    Some((
        Uuid::parse_str(form.device_uuid.trim()).ok()?,
        form.alias.clone(),
        normalize_optional_notes(form.notes.clone()),
        form.position.trim().parse().ok()?,
    ))
}
