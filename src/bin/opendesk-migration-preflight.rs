//! Read-only staging migration preflight.
//!
//! This command never applies a migration, runs schema migrations, or creates a
//! database. It binds a sanitized export to an initialized staging database and a
//! verified backup of that same database instance.

use std::{env, fs, path::PathBuf, process::ExitCode};

use opendesk::domain::{
    migration_contract::parse_sanitized_export, migration_preflight::capture_preflight,
};
use serde::Serialize;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

struct Cli {
    input: PathBuf,
    database: PathBuf,
    backup: PathBuf,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn parse_cli(arguments: impl IntoIterator<Item = String>) -> Result<Cli, ()> {
    let mut input = None;
    let mut database = None;
    let mut backup = None;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--input" if input.is_none() => input = arguments.next().map(PathBuf::from),
            "--database" if database.is_none() => database = arguments.next().map(PathBuf::from),
            "--backup" if backup.is_none() => backup = arguments.next().map(PathBuf::from),
            _ => return Err(()),
        }
    }
    Ok(Cli {
        input: input.ok_or(())?,
        database: database.ok_or(())?,
        backup: backup.ok_or(())?,
    })
}

fn regular_file(path: &PathBuf, limit_bytes: Option<u64>) -> Result<(), ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || limit_bytes.is_some_and(|limit| metadata.len() > limit)
    {
        return Err(());
    }
    Ok(())
}

fn error() -> ExitCode {
    println!(
        "{}",
        serde_json::to_string(&ErrorResponse {
            error: "migration preflight prerequisites failed"
        })
        .expect("static response")
    );
    ExitCode::from(2)
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match parse_cli(env::args().skip(1)) {
        Ok(cli) => cli,
        Err(()) => return error(),
    };
    if regular_file(&cli.input, Some(MAX_INPUT_BYTES)).is_err()
        || regular_file(&cli.database, None).is_err()
        || regular_file(&cli.backup, None).is_err()
    {
        return error();
    }
    let input = match fs::read(&cli.input) {
        Ok(value) => value,
        Err(_) => return error(),
    };
    let export = match std::str::from_utf8(&input)
        .ok()
        .and_then(|value| parse_sanitized_export(value).ok())
    {
        Some(value) => value,
        None => return error(),
    };
    match capture_preflight(&cli.database, &input, &export, &cli.backup).await {
        Ok(preflight) => match serde_json::to_string(&preflight) {
            Ok(value) => {
                println!("{value}");
                ExitCode::SUCCESS
            }
            Err(_) => error(),
        },
        Err(_) => error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_requires_exact_three_paths() {
        assert!(parse_cli(Vec::<String>::new()).is_err());
        assert!(parse_cli(vec!["--input".into(), "export.json".into()]).is_err());
        assert!(parse_cli(vec!["--apply".into()]).is_err());
    }
}
