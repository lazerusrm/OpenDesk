use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Site {
    pub site_uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteDraft {
    pub name: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SiteValidationError {
    #[error("site name must not be empty")]
    EmptyName,
    #[error("site name is too long")]
    NameTooLong,
}

pub fn normalize_site_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn validate_site_draft(draft: &SiteDraft) -> Result<(), SiteValidationError> {
    let name = normalize_site_name(&draft.name);
    if name.is_empty() {
        return Err(SiteValidationError::EmptyName);
    }
    if name.len() > 128 {
        return Err(SiteValidationError::NameTooLong);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_site_draft_rejects_empty_name() {
        let draft = SiteDraft {
            name: "   ".to_string(),
        };
        assert_eq!(
            validate_site_draft(&draft),
            Err(SiteValidationError::EmptyName)
        );
    }

    #[test]
    fn validate_site_draft_accepts_trimmed_name() {
        let draft = SiteDraft {
            name: "  Lab Floor  ".to_string(),
        };
        assert!(validate_site_draft(&draft).is_ok());
        assert_eq!(normalize_site_name(&draft.name), "Lab Floor");
    }
}