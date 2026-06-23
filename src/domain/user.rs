use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub user_uuid: Uuid,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserValidationError {
    #[error("username must not be empty")]
    EmptyUsername,
    #[error("username is too long")]
    UsernameTooLong,
}

pub fn validate_username(username: &str) -> Result<(), UserValidationError> {
    let trimmed = username.trim();
    if trimmed.is_empty() {
        return Err(UserValidationError::EmptyUsername);
    }
    if trimmed.len() > 64 {
        return Err(UserValidationError::UsernameTooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_username_rejects_empty() {
        assert_eq!(
            validate_username("   "),
            Err(UserValidationError::EmptyUsername)
        );
    }

    #[test]
    fn validate_username_accepts_normal_value() {
        assert!(validate_username("admin").is_ok());
    }
}