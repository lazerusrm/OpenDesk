use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const ADDRESS_BOOK_NAME_MAX_LENGTH: usize = 128;
pub const ADDRESS_BOOK_ENTRY_ALIAS_MAX_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddressBook {
    pub address_book_uuid: Uuid,
    pub owner_user_uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddressBookEntry {
    pub address_book_entry_uuid: Uuid,
    pub address_book_uuid: Uuid,
    pub device_uuid: Uuid,
    pub alias: String,
    pub notes: Option<String>,
    pub position: u32,
}

/// Owner-granted official-client access to a shared address book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddressBookAccessRule {
    pub address_book_uuid: Uuid,
    pub principal_type: String,
    pub principal_uuid: Uuid,
    pub permission: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AddressBookValidationError {
    #[error("address book name must not be empty")]
    EmptyName,
    #[error("address book name is too long")]
    NameTooLong,
    #[error("address-book entry alias must not be empty")]
    EmptyAlias,
    #[error("address-book entry alias is too long")]
    AliasTooLong,
}

pub fn normalize_address_book_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn normalize_address_book_entry_alias(alias: &str) -> String {
    alias.trim().to_string()
}

pub fn normalize_optional_notes(notes: Option<String>) -> Option<String> {
    notes
        .map(|notes| notes.trim().to_string())
        .filter(|notes| !notes.is_empty())
}

pub fn validate_address_book_name(name: &str) -> Result<(), AddressBookValidationError> {
    let name = normalize_address_book_name(name);
    if name.is_empty() {
        return Err(AddressBookValidationError::EmptyName);
    }
    if name.len() > ADDRESS_BOOK_NAME_MAX_LENGTH {
        return Err(AddressBookValidationError::NameTooLong);
    }
    Ok(())
}

pub fn validate_address_book_entry_alias(alias: &str) -> Result<(), AddressBookValidationError> {
    let alias = normalize_address_book_entry_alias(alias);
    if alias.is_empty() {
        return Err(AddressBookValidationError::EmptyAlias);
    }
    if alias.len() > ADDRESS_BOOK_ENTRY_ALIAS_MAX_LENGTH {
        return Err(AddressBookValidationError::AliasTooLong);
    }
    Ok(())
}

impl AddressBook {
    pub fn new(
        address_book_uuid: Uuid,
        owner_user_uuid: Uuid,
        name: impl AsRef<str>,
    ) -> Result<Self, AddressBookValidationError> {
        validate_address_book_name(name.as_ref())?;
        Ok(Self {
            address_book_uuid,
            owner_user_uuid,
            name: normalize_address_book_name(name.as_ref()),
        })
    }
}

impl AddressBookEntry {
    pub fn new(
        address_book_entry_uuid: Uuid,
        address_book_uuid: Uuid,
        device_uuid: Uuid,
        alias: impl AsRef<str>,
        notes: Option<String>,
        position: u32,
    ) -> Result<Self, AddressBookValidationError> {
        validate_address_book_entry_alias(alias.as_ref())?;
        Ok(Self {
            address_book_entry_uuid,
            address_book_uuid,
            device_uuid,
            alias: normalize_address_book_entry_alias(alias.as_ref()),
            notes: normalize_optional_notes(notes),
            position,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_trim_canonical_text() {
        let owner = Uuid::new_v4();
        let book = AddressBook::new(Uuid::new_v4(), owner, "  Favorites ").expect("valid book");
        assert_eq!(book.name, "Favorites");
        let entry = AddressBookEntry::new(
            Uuid::new_v4(),
            book.address_book_uuid,
            Uuid::new_v4(),
            "  Workstation ",
            Some("  notes  ".to_string()),
            0,
        )
        .expect("valid entry");
        assert_eq!(entry.alias, "Workstation");
        assert_eq!(entry.notes.as_deref(), Some("notes"));
    }

    #[test]
    fn validation_rejects_empty_and_overlong_names_and_aliases() {
        assert_eq!(
            validate_address_book_name(" "),
            Err(AddressBookValidationError::EmptyName)
        );
        assert_eq!(
            validate_address_book_entry_alias(" "),
            Err(AddressBookValidationError::EmptyAlias)
        );
        assert_eq!(
            validate_address_book_name(&"x".repeat(ADDRESS_BOOK_NAME_MAX_LENGTH + 1)),
            Err(AddressBookValidationError::NameTooLong)
        );
        assert_eq!(
            validate_address_book_entry_alias(&"x".repeat(ADDRESS_BOOK_ENTRY_ALIAS_MAX_LENGTH + 1)),
            Err(AddressBookValidationError::AliasTooLong)
        );
    }
}
