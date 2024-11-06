use std::str::FromStr;

use clap::{Parser, Subcommand};
use error::CliError;
use migrate::MigrateArgs;
use sqlx::{postgres::{PgConnectOptions, PgPoolOptions}, ConnectOptions, Connection, Database, Postgres};
use temp::TempArgs;

use crate::{error::DbToolsError, util::{self, pg::parse_for_maintenance}};

mod migrate;
mod temp;
mod error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct SlArgs {
    /// URL of the main database. Can also be provided by `DATABASE_URL` in the environment. This
    /// flag will take precedence over the environment variable. This should conform to the
    /// typical URL database specification: `protocol://user:pass@host:port/database`
    #[arg(short, long)]
    pub url: Option<String>,

    /// URL of the administrative database, required only for database creation or deletion
    /// actions under certain conditions. If provided, this URL will be used to connect as an
    /// admin for creating and dropping databases. This value can also be provided by the
    /// `DATABASE_ADMIN_URL` environment variable. This flag will take precedence over the
    /// environment variable.
    ///
    /// By default, the system will attempt to use the main database credentials and connect
    /// to `postgres` as the maintenance database. You must provide an admin url if:
    ///
    /// 1. You are creating or dropping a database, AND
    /// 2. Either:
    ///     a. The `postgres` database is not available as a maintenance database, OR
    ///     b. The main database credentials lack permissions to create or drop databases.
    #[arg(short, long)]
    pub admin_url: Option<String>,

    /// Reads in the `.env` file provided prior to running. The database and admin database
    /// URLs can be provided in the environment using `DATABASE_URL` and `DATABASE_ADMIN_URL`,
    /// instead of passing them as command line arguments. You can pass in multiple `.env`
    /// files to read by setting this flag multiple times and it will read them all in the
    /// order provided. By default this will try to read in `.env` in the current directory.
    /// If any `-e` argument is provided it will instead read only the proveded `.env` files,
    /// or if `--no-env` is provided it will read in none of them (even if `-e` is set)
    #[arg(short, long)]
    env: Option<Vec<String>>,

    /// Does not read any `.env` files prior to running the program. By default, it will
    /// attempt to read `.env` in the current directory. Setting this flag will prevent the
    /// program from doing that, as well as prevent it from reading any explicitly-passed
    /// `.env` files from `-e` flags.
    #[arg(short = 'E', long)]
    no_env: bool,

    #[command(subcommand)]
    command: SlSubcommand,

    /// Typically this utility will print out some configuration information letting the
    /// user know what the settings are. This flag will suppress that output. This flag
    /// will NOT bypass user confirmation checks. That is handled by the `-y` flag for
    /// each command that uses it, and must be passed separately.
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum SlSubcommand {
    Migrate(MigrateArgs),
    Temp(TempArgs),
}

impl SlSubcommand {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        match self {
            Self::Migrate(sub_args) => {
                sub_args.run(args).await?;
            },
            Self::Temp(sub_args) => {
                sub_args.run(args)?;
            },
        }
        Ok(())
    }
}

/// The name of the environment variable that holds the database url
const ENV_DATABASE_URL: &str = "DATABASE_URL";
const ENV_DATABASE_URL_ADMIN: &str = "DATABASE_URL_ADMIN";

impl SlArgs {
    /// Gets the main database URL, which will either be provided as a command line argument
    /// or pulled from the environment.
    fn get_url(&self) -> Result<String, CliError> {
        let url = if let Some(url) = &self.url {
            url
        } else {
            &std::env::var(ENV_DATABASE_URL).ok().ok_or(
                CliError::MissingArg(format!("Provide --url or {}", ENV_DATABASE_URL))
            )?
        };
        Ok(url.clone())
    }

    fn get_db_conn_opts(&self) -> Result<PgConnectOptions, CliError> {
        let url = self.get_url()?;
        let conn_opts = PgConnectOptions::from_str(&url).map_err(DbToolsError::from)?;
        Ok(conn_opts)
    }

    fn get_admin_url(&self) -> Result<String, CliError> {
        let admin_url = if let Some(admin_url) = &self.admin_url {
            admin_url.to_owned()
        } else {
            match std::env::var(ENV_DATABASE_URL_ADMIN).ok() {
                Some(url) => url.to_owned(),
                None => {
                    parse_for_maintenance(&self.get_db_conn_opts()?)
                        .to_url_lossy().to_string()
                },
            }
        };
        Ok(admin_url)
    }

    fn get_admin_conn_opts(&self) -> Result<PgConnectOptions, CliError> {
        PgConnectOptions::from_str(&self.get_admin_url()?)
            .map_err(DbToolsError::from)
            .map_err(CliError::from)
    }

    fn print_config(&self) {
        println!("Main Database:  {}", self.get_url().unwrap_or("NONE".to_owned()));
        println!("Admin Database: {}", self.get_admin_url().unwrap_or("NONE".to_owned()));
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        // attempt to read a `.env` file unless explicitly told not to
        if !self.no_env {
            if let Some(env_files) = &self.env {
                // If we were passed a `-e` then only read in the ones passed
                env_files.into_iter().for_each(|fname| { dotenv::from_path(fname).ok(); });
            } else {
                // otherwise attempt to read the standard `.env` file
                let res = dotenv::dotenv()?;//.ok();
                println!("{:?}", res);
            }
        }

        if !self.quiet {
            self.print_config();
        }
        let _ = &self.command.run(self).await?;
        Ok(())
    }
}

