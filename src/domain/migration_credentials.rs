//! Protected credential bridge artifact.
//!
//! This is a narrow external-boundary format. It contains only verified source
//! bcrypt records and provenance; it is never part of the sanitized export.
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const CREDENTIAL_ARTIFACT_SCHEMA_VERSION: u32 = 1;
pub const ACTIVATION_POLICY_ACTIVATE_ALL: &str = "activate_all";

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProtectedCredentialArtifact {
    pub schema_version: u32,
    pub source_system: String,
    pub source_instance: String,
    pub source_export_id: String,
    pub run_id: Uuid,
    pub target_instance_uuid: Uuid,
    pub input_sha256: String,
    pub activation_policy: String,
    pub records: Vec<ProtectedCredentialRecord>,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProtectedCredentialRecord {
    pub source_user_id: String,
    pub verifier_algorithm: String,
    pub verifier: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CredentialArtifactError {
    #[error("credential artifact schema is unsupported")]
    UnsupportedSchema,
    #[error("credential artifact is invalid")]
    Invalid,
    #[error("credential artifact digest is invalid")]
    InvalidDigest,
    #[error("credential artifact activation policy is invalid")]
    InvalidPolicy,
}

impl ProtectedCredentialArtifact {
    pub fn validate(&self) -> Result<(), CredentialArtifactError> {
        if self.schema_version != CREDENTIAL_ARTIFACT_SCHEMA_VERSION {
            return Err(CredentialArtifactError::UnsupportedSchema);
        }
        if self.source_system.trim().is_empty()
            || self.source_instance.trim().is_empty()
            || self.source_export_id.trim().is_empty()
            || self.records.is_empty()
        {
            return Err(CredentialArtifactError::Invalid);
        }
        validate_digest(&self.input_sha256)?;
        if self.activation_policy != ACTIVATION_POLICY_ACTIVATE_ALL {
            return Err(CredentialArtifactError::InvalidPolicy);
        }
        let mut ids = std::collections::HashSet::new();
        for record in &self.records {
            if record.source_user_id.trim().is_empty()
                || !ids.insert(&record.source_user_id)
                || record.verifier_algorithm != "bcrypt"
                || record.verifier.len() != 60
                || !record.verifier.starts_with("$2b$06$")
                || !record.verifier.as_bytes()[7..]
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'.' || *b == b'/')
            {
                return Err(CredentialArtifactError::Invalid);
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CredentialArtifactError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| CredentialArtifactError::Invalid)
    }
}

pub fn parse_artifact(
    value: &[u8],
) -> Result<ProtectedCredentialArtifact, CredentialArtifactError> {
    let artifact: ProtectedCredentialArtifact =
        serde_json::from_slice(value).map_err(|_| CredentialArtifactError::Invalid)?;
    artifact.validate()?;
    Ok(artifact)
}

fn validate_digest(value: &str) -> Result<(), CredentialArtifactError> {
    if value.len() != 64
        || !value.bytes().all(|b| b.is_ascii_hexdigit())
        || value.bytes().any(|b| b.is_ascii_uppercase())
    {
        return Err(CredentialArtifactError::InvalidDigest);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact() -> ProtectedCredentialArtifact {
        ProtectedCredentialArtifact {
            schema_version: 1,
            source_system: "rustdesk_server_pro".into(),
            source_instance: "source".into(),
            source_export_id: "export".into(),
            run_id: Uuid::new_v4(),
            target_instance_uuid: Uuid::new_v4(),
            input_sha256: "a".repeat(64),
            activation_policy: ACTIVATION_POLICY_ACTIVATE_ALL.into(),
            records: vec![ProtectedCredentialRecord {
                source_user_id: "u".into(),
                verifier_algorithm: "bcrypt".into(),
                verifier: bcrypt::hash("synthetic", 6).expect("hash"),
            }],
        }
    }

    #[test]
    fn validates_and_hashes_without_disclosing_verifier() {
        let artifact = artifact();
        let digest = crate::domain::migration_contract::sha256_digest(
            &artifact.canonical_bytes().expect("canonical bytes"),
        );
        assert_eq!(digest.len(), 64);
        assert!(!digest.contains("synthetic"));
    }

    #[test]
    fn rejects_duplicate_or_non_bcrypt_records() {
        let mut artifact = artifact();
        artifact.records.push(artifact.records[0].clone());
        assert_eq!(artifact.validate(), Err(CredentialArtifactError::Invalid));
    }
}
