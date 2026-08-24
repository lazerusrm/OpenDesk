//! Signed, staging-only authorization for a migration apply.
//!
//! The plan is serialized canonically before signing. It binds exact sanitized
//! input bytes and the immutable preflight output; it carries no credentials,
//! source settings, or free-form values.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::migration_preflight::MigrationPreflight;

pub const MIGRATION_APPLY_PLAN_SCHEMA_VERSION: u32 = 1;
const DOMAIN_SEPARATOR: &[u8] = b"opendesk-migration-apply-plan-v1\0";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationApplyPlan {
    pub schema_version: u32,
    pub input_sha256: String,
    pub source_system: String,
    pub source_instance: String,
    pub source_export_id: String,
    pub source_snapshot_sha256: String,
    pub target_instance_uuid: String,
    pub target_snapshot_sha256: String,
    pub backup_instance_uuid: String,
    pub backup_sha256: String,
    pub manifest_sha256: String,
    pub credential_artifact_sha256: Option<String>,
    pub credential_activation_policy: Option<String>,
    pub approved_at: String,
    pub expires_at: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MigrationApplyPlanError {
    #[error("migration apply plan schema is unsupported")]
    UnsupportedSchema,
    #[error("migration apply plan is not canonical")]
    NoncanonicalPlan,
    #[error("migration apply plan has invalid digest")]
    InvalidDigest,
    #[error("migration apply plan has invalid timestamp")]
    InvalidTimestamp,
    #[error("migration apply plan expires before approval")]
    InvalidExpiry,
    #[error("migration apply plan does not match preflight")]
    PreflightMismatch,
    #[error("migration apply plan is not active")]
    NotActive,
    #[error("migration apply plan signature is invalid")]
    InvalidSignature,
    #[error("migration apply plan key material is invalid")]
    InvalidKeyMaterial,
}

impl MigrationApplyPlan {
    pub fn validate_against_preflight(
        &self,
        preflight: &MigrationPreflight,
        now: OffsetDateTime,
    ) -> Result<(), MigrationApplyPlanError> {
        if self.schema_version != MIGRATION_APPLY_PLAN_SCHEMA_VERSION {
            return Err(MigrationApplyPlanError::UnsupportedSchema);
        }
        if self.credential_artifact_sha256.is_some() != self.credential_activation_policy.is_some()
        {
            return Err(MigrationApplyPlanError::PreflightMismatch);
        }
        if let Some(digest) = &self.credential_artifact_sha256 {
            validate_digest(digest)?;
        }
        if let Some(policy) = &self.credential_activation_policy {
            if policy != "activate_all" {
                return Err(MigrationApplyPlanError::PreflightMismatch);
            }
        }
        for digest in [
            &self.input_sha256,
            &self.source_snapshot_sha256,
            &self.target_snapshot_sha256,
            &self.backup_sha256,
            &self.manifest_sha256,
        ] {
            validate_digest(digest)?;
        }
        let approved_at = parse_timestamp(&self.approved_at)?;
        let expires_at = parse_timestamp(&self.expires_at)?;
        if expires_at <= approved_at {
            return Err(MigrationApplyPlanError::InvalidExpiry);
        }
        if now < approved_at || now > expires_at {
            return Err(MigrationApplyPlanError::NotActive);
        }
        if self.input_sha256 != preflight.input_sha256
            || self.source_system != preflight.source_system
            || self.source_instance != preflight.source_instance
            || self.source_export_id != preflight.source_export_id
            || self.source_snapshot_sha256 != preflight.source_snapshot_sha256
            || self.target_instance_uuid != preflight.target_instance_uuid
            || self.target_snapshot_sha256 != preflight.target_snapshot_sha256
            || self.backup_instance_uuid != preflight.backup_instance_uuid
            || self.backup_sha256 != preflight.backup_sha256
        {
            return Err(MigrationApplyPlanError::PreflightMismatch);
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, MigrationApplyPlanError> {
        let bytes =
            serde_json::to_vec(self).map_err(|_| MigrationApplyPlanError::NoncanonicalPlan)?;
        let decoded: MigrationApplyPlan = serde_json::from_slice(&bytes)
            .map_err(|_| MigrationApplyPlanError::NoncanonicalPlan)?;
        if decoded != *self {
            return Err(MigrationApplyPlanError::NoncanonicalPlan);
        }
        let mut output = DOMAIN_SEPARATOR.to_vec();
        output.extend_from_slice(&bytes);
        Ok(output)
    }
}

pub fn verify_plan_signature(
    plan: &MigrationApplyPlan,
    public_key: &[u8],
    signature: &[u8],
) -> Result<(), MigrationApplyPlanError> {
    let key_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| MigrationApplyPlanError::InvalidKeyMaterial)?;
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|_| MigrationApplyPlanError::InvalidKeyMaterial)?;
    let signature =
        Signature::from_slice(signature).map_err(|_| MigrationApplyPlanError::InvalidSignature)?;
    key.verify(&plan.canonical_bytes()?, &signature)
        .map_err(|_| MigrationApplyPlanError::InvalidSignature)
}

fn validate_digest(value: &str) -> Result<(), MigrationApplyPlanError> {
    if value.len() != 64
        || !value.bytes().all(|value| value.is_ascii_hexdigit())
        || value.bytes().any(|value| value.is_ascii_uppercase())
    {
        return Err(MigrationApplyPlanError::InvalidDigest);
    }
    Ok(())
}

fn parse_timestamp(value: &str) -> Result<OffsetDateTime, MigrationApplyPlanError> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|_| MigrationApplyPlanError::InvalidTimestamp)?;
    if parsed.format(&Rfc3339).ok().as_deref() != Some(value) {
        return Err(MigrationApplyPlanError::InvalidTimestamp);
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};
    use time::macros::datetime;

    use super::*;

    fn preflight() -> MigrationPreflight {
        MigrationPreflight {
            schema_version: 1,
            input_sha256: "a".repeat(64),
            source_system: "rustdesk_server_pro".to_string(),
            source_instance: "source-1".to_string(),
            source_export_id: "export-1".to_string(),
            source_snapshot_sha256: "b".repeat(64),
            target_instance_uuid: "11111111-1111-1111-1111-111111111111".to_string(),
            target_snapshot_sha256: "c".repeat(64),
            backup_instance_uuid: "11111111-1111-1111-1111-111111111111".to_string(),
            backup_sha256: "d".repeat(64),
        }
    }

    fn plan(preflight: &MigrationPreflight) -> MigrationApplyPlan {
        MigrationApplyPlan {
            schema_version: 1,
            input_sha256: preflight.input_sha256.clone(),
            source_system: preflight.source_system.clone(),
            source_instance: preflight.source_instance.clone(),
            source_export_id: preflight.source_export_id.clone(),
            source_snapshot_sha256: preflight.source_snapshot_sha256.clone(),
            target_instance_uuid: preflight.target_instance_uuid.clone(),
            target_snapshot_sha256: preflight.target_snapshot_sha256.clone(),
            backup_instance_uuid: preflight.backup_instance_uuid.clone(),
            backup_sha256: preflight.backup_sha256.clone(),
            manifest_sha256: "e".repeat(64),
            credential_artifact_sha256: None,
            credential_activation_policy: None,
            approved_at: "2026-07-22T15:00:00Z".to_string(),
            expires_at: "2026-07-22T16:00:00Z".to_string(),
        }
    }

    #[test]
    fn verifies_signed_active_plan_against_exact_preflight() {
        let preflight = preflight();
        let plan = plan(&preflight);
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let signature = signing_key.sign(&plan.canonical_bytes().expect("canonical plan"));
        plan.validate_against_preflight(&preflight, datetime!(2026-07-22 15:30 UTC))
            .expect("active exact plan");
        verify_plan_signature(
            &plan,
            signing_key.verifying_key().as_bytes(),
            &signature.to_bytes(),
        )
        .expect("valid signature");
    }

    #[test]
    fn rejects_preflight_drift_expiry_and_tampered_signature() {
        let preflight = preflight();
        let mut migration_plan = plan(&preflight);
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let signature =
            signing_key.sign(&migration_plan.canonical_bytes().expect("canonical plan"));
        migration_plan.input_sha256 = "e".repeat(64);
        assert_eq!(
            migration_plan.validate_against_preflight(&preflight, datetime!(2026-07-22 15:30 UTC)),
            Err(MigrationApplyPlanError::PreflightMismatch)
        );
        assert_eq!(
            verify_plan_signature(
                &migration_plan,
                signing_key.verifying_key().as_bytes(),
                &signature.to_bytes(),
            ),
            Err(MigrationApplyPlanError::InvalidSignature)
        );
        let plan = plan(&preflight);
        assert_eq!(
            plan.validate_against_preflight(&preflight, datetime!(2026-07-22 16:01 UTC)),
            Err(MigrationApplyPlanError::NotActive)
        );
    }
}
