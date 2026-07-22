use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const ACCESS_GROUP_NAME_MAX_LENGTH: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessGroup {
    pub access_group_uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AccessGroupValidationError {
    #[error("access group name must not be empty")]
    EmptyName,
    #[error("access group name is too long")]
    NameTooLong,
}

pub fn normalize_access_group_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn validate_access_group_name(name: &str) -> Result<(), AccessGroupValidationError> {
    let name = normalize_access_group_name(name);
    if name.is_empty() {
        return Err(AccessGroupValidationError::EmptyName);
    }
    if name.len() > ACCESS_GROUP_NAME_MAX_LENGTH {
        return Err(AccessGroupValidationError::NameTooLong);
    }
    Ok(())
}

impl AccessGroup {
    pub fn new(
        access_group_uuid: Uuid,
        name: impl AsRef<str>,
    ) -> Result<Self, AccessGroupValidationError> {
        validate_access_group_name(name.as_ref())?;
        Ok(Self {
            access_group_uuid,
            name: normalize_access_group_name(name.as_ref()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_trims_and_validates_name() {
        let group = AccessGroup::new(Uuid::nil(), "  Operators  ").expect("valid name");
        assert_eq!(group.name, "Operators");
    }

    #[test]
    fn validation_rejects_empty_and_overlong_names() {
        assert_eq!(
            validate_access_group_name("  "),
            Err(AccessGroupValidationError::EmptyName)
        );
        assert_eq!(
            validate_access_group_name(&"x".repeat(ACCESS_GROUP_NAME_MAX_LENGTH + 1)),
            Err(AccessGroupValidationError::NameTooLong)
        );
    }
}
