use std::{
    env, fs,
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    process::ExitCode,
    str::FromStr,
};

use opendesk::{auth::password_meets_policy, repository::users::reset_user_password};
use serde::Serialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use time::OffsetDateTime;

const O_CLOEXEC: i32 = 0x80000;
const O_NOFOLLOW: i32 = 0x20000;

unsafe extern "C" {
    fn geteuid() -> u32;
}

struct Cli {
    database: PathBuf,
    username: String,
}

#[derive(Serialize)]
struct SuccessResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
    category: &'static str,
}

fn parse_cli(arguments: impl IntoIterator<Item = String>) -> Result<Cli, ()> {
    let mut database = None;
    let mut username = None;
    let mut reset = false;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--database" if database.is_none() => database = arguments.next().map(PathBuf::from),
            "--username" if username.is_none() => username = arguments.next(),
            "--reset" if !reset => reset = true,
            _ => return Err(()),
        }
    }
    let username = username.ok_or(())?;
    if !reset || username.is_empty() {
        return Err(());
    }
    Ok(Cli {
        database: database.ok_or(())?,
        username,
    })
}

fn validate_database(path: &Path) -> Result<fs::File, ()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_NOFOLLOW | O_CLOEXEC)
        .open(path)
        .map_err(|_| ())?;
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.file_type().is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(());
    }
    Ok(file)
}

fn error(category: &'static str) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string(&ErrorResponse {
            error: "password reset failed",
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
    let _database_guard = match validate_database(&cli.database) {
        Ok(file) => file,
        Err(()) => return error("database"),
    };
    let password = match terminal_input::prompt_password("New password: ") {
        Ok(value) => value,
        Err(_) => return error("terminal"),
    };
    let confirmation = match terminal_input::prompt_password("Confirm new password: ") {
        Ok(value) => value,
        Err(_) => return error("terminal"),
    };
    if password != confirmation {
        return error("confirmation");
    }
    if !password_meets_policy(&password) {
        return error("password_policy");
    }
    let password = password.trim();
    let database_uri = format!("file:/proc/self/fd/{}?mode=rw", _database_guard.as_raw_fd());
    let options = match SqliteConnectOptions::from_str(&database_uri) {
        Ok(options) => options.create_if_missing(false),
        Err(_) => return error("database"),
    };
    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
    {
        Ok(pool) => pool,
        Err(_) => return error("database"),
    };
    match reset_user_password(&pool, &cli.username, &password, OffsetDateTime::now_utc()).await {
        Ok(()) => {
            println!(
                "{}",
                serde_json::to_string(&SuccessResponse { status: "reset" })
                    .expect("static response")
            );
            ExitCode::SUCCESS
        }
        Err(_) => error("reset"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_accepts_only_the_explicit_reset_contract() {
        let accepted = vec![
            "--database".into(),
            "database.sqlite".into(),
            "--username".into(),
            "ExactUser".into(),
            "--reset".into(),
        ];
        assert_eq!(parse_cli(accepted).expect("accepted").username, "ExactUser");
        assert!(parse_cli(vec![
            "--database".into(),
            "database.sqlite".into(),
            "--username".into(),
            "user".into(),
            "--password".into(),
            "secret".into(),
            "--reset".into(),
        ])
        .is_err());
        assert!(parse_cli(vec![
            "--database".into(),
            "database.sqlite".into(),
            "--username".into(),
            "user".into(),
        ])
        .is_err());
    }
}
