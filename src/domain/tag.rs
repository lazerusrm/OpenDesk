use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub tag_uuid: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TagDraft {
    pub name: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TagValidationError {
    #[error("tag name must not be empty")]
    EmptyName,
    #[error("tag name is too long")]
    NameTooLong,
}

pub fn normalize_tag_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn validate_tag_draft(draft: &TagDraft) -> Result<(), TagValidationError> {
    let name = normalize_tag_name(&draft.name);
    if name.is_empty() {
        return Err(TagValidationError::EmptyName);
    }
    if name.len() > 128 {
        return Err(TagValidationError::NameTooLong);
    }
    Ok(())
}

pub fn format_tag_names_display(tag_names: &[String]) -> String {
    if tag_names.is_empty() {
        "-".to_string()
    } else {
        tag_names.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_tag_draft_rejects_empty_name() {
        let draft = TagDraft {
            name: "   ".to_string(),
        };
        assert_eq!(
            validate_tag_draft(&draft),
            Err(TagValidationError::EmptyName)
        );
    }

    #[test]
    fn format_tag_names_display_joins_names() {
        assert_eq!(
            format_tag_names_display(&["ops".to_string(), "lab".to_string()]),
            "ops, lab"
        );
        assert_eq!(format_tag_names_display(&[]), "-");
    }
}