use clap::{Args, Subcommand};
use log::info;

use super::SlArgs;

/// Make a new temporary database
///
/// Useful to make a backup of the current main database or to spin off a new
/// playground database.
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

    /// Omit the timestamp from the name
    ///
    /// By default the new database name will contain a timestamp in the format `YYYYmmddHHMMSS`.
    /// This flag will remove that timestamp:
    ///
    /// 202407101124_base_name_UUID
    ///
    /// ^^^^^^^^^^^^ Removes this
    #[arg(short = 'T', long)]
    no_timestamp: bool,

    /// Omit the UUID from the name
    ///
    /// By default the new database name will contain a UUID appended to the end, just to
    /// resolve any potential naming conflicts. This flag will remove that UUID
    ///
    /// 202407101124_base_name_UUID
    ///
    /// _________ Removes this ^^^^
    #[arg(short = 'U', long)]
    no_uuid: bool,
}

/// Clean up or list existing temporary databases
///
/// Temporary databases might hang around if a test fails or neglects to clean up
/// the database it created, and this just provides a quick utility to remove them.
#[derive(Args, Debug, Clone)]
pub struct TempClean {
    /// Do not confirm prior to removing temp databases
    #[arg(short, long)]
    pub yes: bool,
}

/// View the current temporary databases that exist on the server
#[derive(Args, Debug, Clone)]
pub struct TempList {
    /// Use this if your temporary databases are created with no timestamp
    #[arg(short = 'T', long)]
    no_timestamp: bool,

    /// Use this if you want to find temporary databases with a specific name
    #[arg(short, long)]
    name: Option<String>,
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
    pub fn run(&self, _args: &SlArgs) -> anyhow::Result<()> {
        match &self.command {
            TempCommand::Clean(_sub_args) => {
                info!("Cleaning...");
            },
            TempCommand::Create(_sub_args) => {
                info!("Creating...");
            },
            TempCommand::List(_sub_args) => {
                info!("Listing...");
            },
        }
        Ok(())
    }
}
