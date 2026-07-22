//! Read-only operator report for a sanitized external fleet export.
//!
//! This binary has no write/import operation. It reads an existing SQLite
//! database through an immutable, read-only connection, validates explicit
//! group-to-site mappings, and prints only the reconciliation report as JSON.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use opendesk::domain::migration::{
    dry_run_import, load_migration_snapshot, parse_rustdesk_pro_import_json,
    parse_scope_site_mapping, validate_scope_site_mappings, ScopeSiteMapping,
};
use serde::Serialize;
use sqlx::{sqlite::SqliteConnectOptions, Connection};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
struct Cli {
    input: PathBuf,
    database: PathBuf,
    mappings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CliError {
    error: &'static str,
}

fn usage() -> &'static str {
    "usage: opendesk-migration-dry-run --input FILE --database SQLITE_FILE [--map GROUP_ID:SITE_UUID]..."
}

fn parse_cli<I>(arguments: I) -> Result<Cli, &'static str>
where
    I: IntoIterator<Item = String>,
{
    let mut input = None;
    let mut database = None;
    let mut mappings = Vec::new();
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--input" => {
                if input.is_some() {
                    return Err("duplicate --input");
                }
                input = Some(arguments.next().ok_or("--input requires a file")?);
            }
            "--database" => {
                if database.is_some() {
                    return Err("duplicate --database");
                }
                database = Some(
                    arguments
                        .next()
                        .ok_or("--database requires a SQLite file")?,
                );
            }
            "--map" => mappings.push(
                arguments
                    .next()
                    .ok_or("--map requires GROUP_ID:SITE_UUID")?,
            ),
            "--help" | "-h" => return Err(usage()),
            _ => return Err("unknown argument"),
        }
    }
    Ok(Cli {
        input: input.ok_or("--input is required")?.into(),
        database: database.ok_or("--database is required")?.into(),
        mappings,
    })
}

fn regular_file(path: &Path, label: &'static str) -> Result<fs::Metadata, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| label)?;
    if !metadata.file_type().is_file() {
        return Err(label);
    }
    Ok(metadata)
}

fn wal_sidecar_is_safe(database: &Path) -> Result<(), &'static str> {
    let wal_path = PathBuf::from(format!("{}-wal", database.display()));
    let metadata = match fs::symlink_metadata(&wal_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("database WAL sidecar cannot be inspected"),
    };
    if !metadata.file_type().is_file() {
        return Err("database WAL sidecar is unsafe; use a verified SQLite backup artifact");
    }
    if metadata.len() > 0 {
        return Err("database has an active WAL sidecar; use a verified SQLite backup artifact");
    }
    Ok(())
}
fn print_error(error: &'static str) -> ExitCode {
    // Keep failures machine-readable and intentionally omit paths and backend
    // details, which could disclose local topology or credentials.
    println!(
        "{}",
        serde_json::to_string(&CliError { error }).expect("static error serializes")
    );
    ExitCode::from(2)
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = match parse_cli(env::args().skip(1)) {
        Ok(cli) => cli,
        Err(_) => return print_error("invalid command line; see documented usage"),
    };
    let input_metadata = match regular_file(&cli.input, "invalid input file") {
        Ok(metadata) => metadata,
        Err(error) => return print_error(error),
    };
    if input_metadata.len() > MAX_INPUT_BYTES {
        return print_error("input file is too large");
    }
    let database_metadata = match regular_file(&cli.database, "invalid database file") {
        Ok(metadata) => metadata,
        Err(error) => return print_error(error),
    };
    if database_metadata.len() == 0 {
        return print_error("database file is empty");
    }
    if let Err(error) = wal_sidecar_is_safe(&cli.database) {
        return print_error(error);
    }

    let input = match fs::read_to_string(&cli.input) {
        Ok(input) => input,
        Err(_) => return print_error("input file cannot be read as UTF-8"),
    };
    let document = match parse_rustdesk_pro_import_json(&input) {
        Ok(document) => document,
        Err(_) => return print_error("input is not a valid sanitized export"),
    };
    let mappings: Vec<ScopeSiteMapping> = match cli
        .mappings
        .iter()
        .map(|mapping| parse_scope_site_mapping(mapping))
        .collect()
    {
        Ok(mappings) => mappings,
        Err(_) => return print_error("mapping must be GROUP_ID:SITE_UUID"),
    };

    let options = SqliteConnectOptions::new()
        .filename(&cli.database)
        .read_only(true)
        .immutable(true)
        .create_if_missing(false);
    let mut connection = match sqlx::SqliteConnection::connect_with(&options).await {
        Ok(connection) => connection,
        Err(_) => return print_error("current database cannot be opened read-only"),
    };
    let mut snapshot = match load_migration_snapshot(&mut connection).await {
        Ok(snapshot) => snapshot,
        Err(_) => return print_error("current database snapshot is invalid"),
    };
    if validate_scope_site_mappings(&document, &snapshot, &mappings).is_err() {
        return print_error("mapping references unknown or duplicate data");
    }
    snapshot.scope_site_mappings = mappings;

    let report = dry_run_import(&document, &snapshot);
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(_) => print_error("report could not be serialized"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Connection, Executor};

    fn temporary_database_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "opendesk-migration-test-{}.sqlite",
            uuid::Uuid::new_v4()
        ))
    }

    #[tokio::test]
    async fn immutable_connection_cannot_write() {
        let path = temporary_database_path();
        let mut writable = sqlx::SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .expect("create test database");
        writable
            .execute("CREATE TABLE marker (value TEXT NOT NULL)")
            .await
            .expect("create marker");
        writable.close().await.expect("close writable connection");

        let immutable_options = SqliteConnectOptions::new()
            .filename(&path)
            .read_only(true)
            .immutable(true)
            .create_if_missing(false);
        let mut immutable = sqlx::SqliteConnection::connect_with(&immutable_options)
            .await
            .expect("open immutable database");
        let result = immutable
            .execute("INSERT INTO marker (value) VALUES ('x')")
            .await;
        assert!(result.is_err());
        immutable.close().await.expect("close immutable connection");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn wal_sidecar_rejects_active_nonempty_file() {
        let path = temporary_database_path();
        let wal_path = PathBuf::from(format!("{}-wal", path.display()));
        fs::write(&wal_path, b"active wal").expect("create WAL sidecar");
        assert_eq!(
            wal_sidecar_is_safe(&path),
            Err("database has an active WAL sidecar; use a verified SQLite backup artifact")
        );
        fs::remove_file(wal_path).expect("remove WAL sidecar");
    }

    #[test]
    fn wal_sidecar_rejects_nonregular_file() {
        let path = temporary_database_path();
        let wal_path = PathBuf::from(format!("{}-wal", path.display()));
        fs::create_dir(&wal_path).expect("create WAL directory");
        assert_eq!(
            wal_sidecar_is_safe(&path),
            Err("database WAL sidecar is unsafe; use a verified SQLite backup artifact")
        );
        fs::remove_dir(wal_path).expect("remove WAL directory");
    }

    #[test]
    fn wal_sidecar_allows_missing_file() {
        let path = temporary_database_path();
        assert!(wal_sidecar_is_safe(&path).is_ok());
    }

    #[test]
    fn cli_requires_input_and_database() {
        assert_eq!(parse_cli(Vec::<String>::new()), Err("--input is required"));
        assert_eq!(
            parse_cli(vec!["--input".to_string(), "export.json".to_string()]),
            Err("--database is required")
        );
    }

    #[test]
    fn cli_rejects_unknown_and_duplicate_options() {
        assert_eq!(
            parse_cli(vec!["--unknown".to_string()]),
            Err("unknown argument")
        );
        assert_eq!(
            parse_cli(vec![
                "--input".to_string(),
                "a".to_string(),
                "--input".to_string(),
                "b".to_string()
            ]),
            Err("duplicate --input")
        );
    }
}
