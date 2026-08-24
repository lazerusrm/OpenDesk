//! Explicit staging-only migration apply command.
//!
//! The command cannot create a database or run schema migrations. It requires a
//! signed plan that binds the exact sanitized export, manifest, target, and backup.

use std::{
    env, fs,
    os::unix::{fs::MetadataExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
    process::ExitCode,
};

use opendesk::{
    domain::{
        migration_apply_plan::MigrationApplyPlan,
        migration_contract::{parse_sanitized_export, MigrationManifest},
        migration_preflight::MigrationPreflight,
    },
    repository::migration_apply::{apply_staging_migration_with_credentials, MigrationApplyError},
};
use serde::Serialize;
use time::OffsetDateTime;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

unsafe extern "C" {
    fn geteuid() -> u32;
}

struct Cli {
    input: PathBuf,
    manifest: PathBuf,
    preflight: PathBuf,
    plan: PathBuf,
    signature: PathBuf,
    database: PathBuf,
    backup: PathBuf,
    credentials: Option<PathBuf>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    category: &'static str,
}

fn parse_cli(arguments: impl IntoIterator<Item = String>) -> Result<Cli, ()> {
    let mut input = None;
    let mut manifest = None;
    let mut preflight = None;
    let mut plan = None;
    let mut signature = None;
    let mut database = None;
    let mut backup = None;
    let mut credentials = None;
    let mut staging = false;
    let mut apply = false;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--staging" if !staging => staging = true,
            "--apply" if !apply => apply = true,
            "--input" if input.is_none() => input = arguments.next().map(PathBuf::from),
            "--manifest" if manifest.is_none() => manifest = arguments.next().map(PathBuf::from),
            "--preflight" if preflight.is_none() => preflight = arguments.next().map(PathBuf::from),
            "--plan" if plan.is_none() => plan = arguments.next().map(PathBuf::from),
            "--signature" if signature.is_none() => signature = arguments.next().map(PathBuf::from),
            "--database" if database.is_none() => database = arguments.next().map(PathBuf::from),
            "--backup" if backup.is_none() => backup = arguments.next().map(PathBuf::from),
            "--credentials" if credentials.is_none() => {
                credentials = arguments.next().map(PathBuf::from)
            }
            _ => return Err(()),
        }
    }
    if !staging || !apply {
        return Err(());
    }
    Ok(Cli {
        input: input.ok_or(())?,
        manifest: manifest.ok_or(())?,
        preflight: preflight.ok_or(())?,
        plan: plan.ok_or(())?,
        signature: signature.ok_or(())?,
        database: database.ok_or(())?,
        backup: backup.ok_or(())?,
        credentials,
    })
}

fn read_regular(path: &Path, limit: Option<u64>) -> Result<Vec<u8>, ()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x20000 | 0x80000)
        .open(path)
        .map_err(|_| ())?;
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { geteuid() }
        || metadata.mode() & 0o077 != 0
        || metadata.len() == 0
        || limit.is_some_and(|value| metadata.len() > value)
    {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    std::io::Read::read_to_end(&mut std::io::BufReader::new(file), &mut bytes).map_err(|_| ())?;
    Ok(bytes)
}

fn configured_verifying_key() -> Result<Vec<u8>, ()> {
    let value = env::var("OPENDESK_MIGRATION_APPROVAL_PUBLIC_KEY_HEX").map_err(|_| ())?;
    let key = hex::decode(value).map_err(|_| ())?;
    if key.len() != 32 {
        return Err(());
    }
    Ok(key)
}

fn error(category: &'static str) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string(&ErrorResponse {
            error: "migration apply prerequisites failed",
            category,
        })
        .expect("static error")
    );
    ExitCode::from(2)
}

fn apply_error_category(error: &MigrationApplyError) -> &'static str {
    match error {
        MigrationApplyError::Guard(error) => match error {
            opendesk::domain::migration_apply_guard::MigrationApplyGuardError::Approval(_) => "approval",
            opendesk::domain::migration_apply_guard::MigrationApplyGuardError::BackupChanged => "backup",
            opendesk::domain::migration_apply_guard::MigrationApplyGuardError::TargetChanged
            | opendesk::domain::migration_apply_guard::MigrationApplyGuardError::TargetIdentityChanged => "target",
            opendesk::domain::migration_apply_guard::MigrationApplyGuardError::TargetNotStaging => "staging_target",
            _ => "guard",
        },
        MigrationApplyError::ManifestMismatch | MigrationApplyError::Contract(_) => "contract",
        MigrationApplyError::UnsupportedSemantics => "unsupported_semantics",
        MigrationApplyError::Collision | MigrationApplyError::Replay => "collision",
        MigrationApplyError::InvalidRole => "source_role",
        MigrationApplyError::Database(_) => "database",
        MigrationApplyError::Credentials(_)
        | MigrationApplyError::CredentialSetMismatch
        | MigrationApplyError::CredentialBindingMismatch
        | MigrationApplyError::CredentialRunMismatch
        | MigrationApplyError::CredentialBindingMissing
        | MigrationApplyError::CredentialArtifactRequired => "credentials",
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match parse_cli(env::args().skip(1)) {
        Ok(value) => value,
        Err(()) => return error("arguments"),
    };
    let input = match read_regular(&cli.input, Some(MAX_INPUT_BYTES)) {
        Ok(value) => value,
        Err(()) => return error("arguments"),
    };
    let manifest = match read_regular(&cli.manifest, Some(MAX_INPUT_BYTES))
        .ok()
        .and_then(|value| serde_json::from_slice::<MigrationManifest>(&value).ok())
    {
        Some(value) => value,
        None => return error("artifact"),
    };
    let preflight = match read_regular(&cli.preflight, Some(MAX_INPUT_BYTES))
        .ok()
        .and_then(|value| serde_json::from_slice::<MigrationPreflight>(&value).ok())
    {
        Some(value) => value,
        None => return error("artifact"),
    };
    let plan = match read_regular(&cli.plan, Some(MAX_INPUT_BYTES))
        .ok()
        .and_then(|value| serde_json::from_slice::<MigrationApplyPlan>(&value).ok())
        .filter(|value| {
            value.schema_version
                == opendesk::domain::migration_apply_plan::MIGRATION_APPLY_PLAN_SCHEMA_VERSION
        }) {
        Some(value) => value,
        None => return error("artifact"),
    };
    let signature = match read_regular(&cli.signature, Some(1024)) {
        Ok(value) => value,
        Err(()) => return error("arguments"),
    };
    let public_key = match configured_verifying_key() {
        Ok(value) => value,
        Err(()) => return error("arguments"),
    };
    if read_regular(&cli.database, None).is_err() || read_regular(&cli.backup, None).is_err() {
        return error("artifact");
    }
    let credentials = match cli.credentials.as_ref() {
        None => None,
        Some(path) => match read_regular(path, Some(MAX_INPUT_BYTES)) {
            Ok(value) => Some(value),
            Err(()) => return error("credentials"),
        },
    };
    let export = match std::str::from_utf8(&input)
        .ok()
        .and_then(|value| parse_sanitized_export(value).ok())
    {
        Some(value) => value,
        None => return error("artifact"),
    };
    match apply_staging_migration_with_credentials(
        &cli.database,
        &cli.backup,
        &input,
        &export,
        &manifest,
        &preflight,
        &plan,
        &public_key,
        &signature,
        OffsetDateTime::now_utc(),
        credentials.as_deref(),
    )
    .await
    {
        Ok(()) => {
            println!(
                "{{\"status\":\"applied\",\"run_id\":\"{}\"}}",
                export.run.run_id
            );
            ExitCode::SUCCESS
        }
        Err(apply_error) => error(apply_error_category(&apply_error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_key_requires_exact_hex_length() {
        unsafe {
            env::remove_var("OPENDESK_MIGRATION_APPROVAL_PUBLIC_KEY_HEX");
        }
        assert!(configured_verifying_key().is_err());
        unsafe {
            env::set_var("OPENDESK_MIGRATION_APPROVAL_PUBLIC_KEY_HEX", "00");
        }
        assert!(configured_verifying_key().is_err());
        unsafe {
            env::set_var(
                "OPENDESK_MIGRATION_APPROVAL_PUBLIC_KEY_HEX",
                "00".repeat(32),
            );
        }
        assert_eq!(configured_verifying_key().expect("key").len(), 32);
        unsafe {
            env::remove_var("OPENDESK_MIGRATION_APPROVAL_PUBLIC_KEY_HEX");
        }
    }

    #[test]
    fn cli_requires_explicit_staging_and_apply_flags() {
        assert!(parse_cli(Vec::<String>::new()).is_err());
        assert!(parse_cli(vec![
            "--staging".into(),
            "--apply".into(),
            "--unknown".into()
        ])
        .is_err());
    }
}
