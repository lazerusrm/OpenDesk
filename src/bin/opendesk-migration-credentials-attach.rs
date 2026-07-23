use std::{
    env, fs,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::ExitCode,
};

use opendesk::repository::migration_credentials::attach_protected_credentials;
use serde::Serialize;
use time::OffsetDateTime;

const MAX_CREDENTIAL_BYTES: u64 = 16 * 1024 * 1024;

unsafe extern "C" {
    fn geteuid() -> u32;
}

struct Cli {
    database: PathBuf,
    credentials: PathBuf,
    expected_sha256: String,
}

#[derive(Serialize)]
struct Response {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    category: &'static str,
}

fn parse_cli(arguments: impl IntoIterator<Item = String>) -> Result<Cli, ()> {
    let mut attach = false;
    let mut activate_all = false;
    let mut database = None;
    let mut credentials = None;
    let mut expected_sha256 = None;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--attach" if !attach => attach = true,
            "--activate-all" if !activate_all => activate_all = true,
            "--database" if database.is_none() => database = arguments.next().map(PathBuf::from),
            "--credentials" if credentials.is_none() => {
                credentials = arguments.next().map(PathBuf::from)
            }
            "--expected-sha256" if expected_sha256.is_none() => expected_sha256 = arguments.next(),
            _ => return Err(()),
        }
    }
    if !attach || !activate_all {
        return Err(());
    }
    Ok(Cli {
        database: database.ok_or(())?,
        credentials: credentials.ok_or(())?,
        expected_sha256: expected_sha256.ok_or(())?,
    })
}

fn read_credentials(path: &Path) -> Result<Vec<u8>, ()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x20000 | 0x80000)
        .open(path)
        .map_err(|_| ())?;
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { geteuid() }
        || metadata.mode() & 0o777 != 0o600
        || metadata.len() == 0
        || metadata.len() > MAX_CREDENTIAL_BYTES
    {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    std::io::Read::read_to_end(&mut std::io::BufReader::new(file), &mut bytes).map_err(|_| ())?;
    Ok(bytes)
}

fn error(category: &'static str) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string(&ErrorResponse {
            error: "credential attachment failed",
            category,
        })
        .expect("static response")
    );
    ExitCode::from(2)
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match parse_cli(env::args().skip(1)) {
        Ok(value) => value,
        Err(()) => return error("arguments"),
    };
    let credentials = match read_credentials(&cli.credentials) {
        Ok(value) => value,
        Err(()) => return error("artifact"),
    };
    match attach_protected_credentials(
        &cli.database,
        &credentials,
        &cli.expected_sha256,
        OffsetDateTime::now_utc(),
    )
    .await
    {
        Ok(()) => {
            println!(
                "{}",
                serde_json::to_string(&Response { status: "attached" }).expect("static response")
            );
            ExitCode::SUCCESS
        }
        Err(_) => error("attachment"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_requires_both_explicit_actions() {
        assert!(parse_cli(Vec::<String>::new()).is_err());
        assert!(parse_cli(vec![
            "--attach".into(),
            "--activate-all".into(),
            "--database".into(),
            "db".into(),
            "--credentials".into(),
            "credentials".into(),
            "--expected-sha256".into(),
            "a".repeat(64),
        ])
        .is_ok());
    }
}
