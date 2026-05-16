use clap::{Args, Subcommand};
use log::info;

use crate::{db::pg::{manager::PgManagerDb, temp::{Initial, PgTempDbBuilder}}, managed::ManagedDb, manager::ManagerDb, namer::DbNamingProps};

use super::SlArgs;

/// Make a new temporary database
///
/// Useful to make a backup of the current main database or to spin off a new
/// playground database. This will use the `TEMP_DATABASE_PATTERN` when generating
/// database names.
#[derive(Args, Debug, Clone)]
pub struct TempCreate {
    /// Set base name of the database
    ///
    /// Overrides the base name of the database. By default the new database name will use the
    /// base name of the main database.
    ///
    /// 202407101124_base_name_UUID
    ///
    /// _____________^^^^ Sets this
    #[arg(short, long)]
    base: Option<String>,

    /// Extra name to append to base name when creating database
    ///
    /// Extra name to append to the database
    ///
    /// 202407101124_base_name_UUID
    ///
    /// __________________^^^^ adds this
    #[arg(short, long)]
    name: Option<String>,

    /// Create an empty database
    ///
    /// By default the new database name will be created as a copy of the current main database.
    /// Pass this flag to create a completely blank database.
    #[arg(short, long)]
    empty: bool,
}

/// Clean up or list existing temporary databases
///
/// Temporary databases might hang around if a test fails or neglects to clean up
/// the database it created, and this just provides a quick utility to remove them.
#[derive(Args, Debug, Clone)]
pub struct TempClean {
    /// Auto-confirm cleanup
    #[arg(short, long)]
    pub yes: bool,

    /// If set, matches any database `base` name
    ///
    /// When matching by pattern, by default it will only find temporary databases
    /// that match the current `DATABASE_URL` base name. Passing this will find
    /// any that match the general pattern, regardless of the base name.
    #[arg(short, long)]
    pub any: bool,
}

/// View the current temporary databases that exist on the server
#[derive(Args, Debug, Clone)]
pub struct TempList {
    /// Use this if your temporary databases are created with no timestamp
    #[arg(short = 'T', long)]
    no_timestamp: bool,

    /// If set, matches any database `base` name
    ///
    /// When matching by pattern, by default it will only find temporary databases
    /// that match the current `DATABASE_URL` base name. Passing this will find
    /// any that match the general pattern, regardless of the base name.
    #[arg(short, long)]
    pub any: bool,
}

/// Manages the temporary databases created on the server
///
/// Primarily these are generated from running tests and are identified by the common
/// database name structure that they are given during that process, consisting of
/// a "base" name, derived from the passed-in URL, along with timestamps, uuids, and
/// additional strings to avoid naming conflicts
#[derive(Subcommand, Debug, Clone)]
pub enum TempCommand {
    Clean(TempClean),
    Create(TempCreate),
    List(TempList),
}

/// Utilities for managing temporary databases
#[derive(Args, Debug, Clone)]
pub struct TempArgs {
    #[command(subcommand)]
    command: TempCommand,
}

impl TempArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        match &self.command {
            TempCommand::Clean(sub_args) => {
                info!("Cleaning...");
                let passed_regex = args.get_temp_db_regex();
                let regex = if let Some(regex) = passed_regex {
                    regex
                } else {
                    let pattern = args.get_temp_db_pattern();
                    info!(
                        "No temp database regex passed; inferring from pattern: {}",
                        pattern.into_pattern()
                    );
                    let db_base = args.get_url()?;
                    DbNamingProps {
                        pattern,
                        base: if sub_args.any { None } else { Some(db_base.dbname().to_owned()) },
                        name: None,
                        keep_full: false,
                    }.into_regex()
                };
                info!("Cleaning with regex /{}/", regex);
                let manager_url = args.get_admin_url()?;
                let manager = PgManagerDb::new(manager_url.clone())?;
                let all_found = manager.find_by_regex(&regex).await?;
                if all_found.is_empty() {
                    info!("No temporary databases found; nothing to do");
                    return Ok(());
                }
                info!("Found {} temporary databases:", all_found.len());
                all_found.iter().for_each(|f| println!("{}", f.url().dbname()));
                if !sub_args.yes {
                    print!("Drop {} databases? [y/N] ", all_found.len());
                    std::io::Write::flush(&mut std::io::stdout())?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().to_lowercase() != "y" {
                        info!("Aborted.");
                        return Ok(());
                    }
                }
                for db in all_found {
                    db.drop().await?;
                }
            },
            TempCommand::Create(sub_args) => {
                info!("Creating...");
                let base_url = args.get_url()?;
                let new_base_url = if let Some(new_base) = &sub_args.base {
                    let mut new_url = base_url.clone();
                    new_url.set_dbname(new_base);
                    new_url
                } else { base_url };
                let mut temp_builder = PgTempDbBuilder::new(
                    &new_base_url,
                    &Some(args.get_admin_url()?),
                    if sub_args.empty {
                        Initial::Empty
                    } else { Initial::Template(args.get_url()?) },
                    args.get_temp_db_pattern(),
                )?;

                if let Some(name) = &sub_args.name {
                    temp_builder = temp_builder.set_name(name.to_owned())
                }
                let created_db = temp_builder.build().await?;
                info!("Created Temp Database: {:?}", created_db.url().as_str());
            },
            TempCommand::List(sub_args) => {
                let passed_regex = args.get_temp_db_regex();
                let regex = if let Some(regex) = passed_regex {
                    regex
                } else {
                    let pattern = args.get_temp_db_pattern();
                    info!(
                        "No temp database regex passed; inferring from pattern: {}",
                        pattern.into_pattern()
                    );
                    let db_base = args.get_url()?;
                    DbNamingProps {
                        pattern,
                        base: if sub_args.any { None } else { Some(db_base.dbname().to_owned()) },
                        name: None,
                        keep_full: false,
                    }.into_regex()
                };
                info!("Listing with regex /{}/", regex);
                let manager_url = args.get_admin_url()?;
                let manager = PgManagerDb::new(manager_url.clone())?;
                let all_found = manager.find_by_regex(&regex).await?;
                info!("Found {} temporary databases:", all_found.len());
                all_found.iter().for_each(|f| println!("{}", f.url().dbname()));
            },
        }
        Ok(())
    }
}
