use clap::{Args, Subcommand};

use super::SlArgs;

/// Make a new temporary database. Useful to make a backup of the current main database
/// or to spin off a new playground database.
#[derive(Args, Debug, Clone)]
pub struct TempCreate {
    /// Overrides the base name of the database. By default the new database name will use the
    /// base name of the main database.
    ///
    /// 202407101124_base_name_UUID
    ///
    /// _____________^^^^ Sets this
    #[arg(short, long)]
    base: Option<String>,

    /// Extra name to append to the database
    ///
    /// 202407101124_base_name_UUID
    ///
    /// __________________^^^^ adds this
    #[arg(short, long)]
    name: Option<String>,

    /// By default the new database name will be created as a copy of the current main database.
    /// Pass this flag to create a completely blank database.
    #[arg(short, long)]
    empty: bool,

    /// By default the new database name will contain a timestamp in the format `YYYYmmddHHMMSS`.
    /// This flag will remove that timestamp:
    ///
    /// 202407101124_base_name_UUID
    ///
    /// ^^^^^^^^^^^^ Removes this
    #[arg(short = 'T', long)]
    no_timestamp: bool,

    /// By default the new database name will contain a UUID appended to the end, just to
    /// resolve any potential naming conflicts. This flag will remove that UUID
    ///
    /// 202407101124_base_name_UUID
    ///
    /// _________ Removes this ^^^^
    #[arg(short = 'U', long)]
    no_uuid: bool,
}

/// Clean up and list the existing temporary databases that use the main database as
/// a base name. Temporary databases might hang around if a test fails or neglects to clean up
/// the database it created, and this just provides a quick utility to remove them.
#[derive(Args, Debug, Clone)]
pub struct TempClean {
    /// Do not confirm prior to removing temp databases
    #[arg(short, long)]
    pub yes: bool,
}

/// Manages the temporary databases created on the server. Primarily these are generated from
/// running tests.
#[derive(Subcommand, Debug, Clone)]
pub enum TempCommand {
    Clean(TempClean),
    Create(TempCreate),
}

/// Utilities for managing temporary databases
#[derive(Args, Debug, Clone)]
pub struct TempArgs {
    #[command(subcommand)]
    command: TempCommand,
}

impl TempArgs {
    pub fn run(&self, args: &SlArgs) {
        match &self.command {
            TempCommand::Clean(sub_args) => {
                println!("Cleaning...");
            },
            TempCommand::Create(sub_args) => {
                println!("Creating...");
            }
        }
    }
}
