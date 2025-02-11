use crate::error::DbToolsError;

use super::{
    step::MigrationStep,
    version::SchemaVersion,
};

pub trait MigrationManager {
    /// Retrieves the current schema version from the database
    fn get_version(&self) -> &SchemaVersion;
    /// Returns a string representing a cli-compatible printout of the current migration
    /// status.
    fn get_summary_str(&self) -> String;
    /// Performs the specified migration. It should do this within the context of a single
    /// transaction, which rolls back if there is an error.
    #[allow(async_fn_in_trait)]
    async fn do_next_migration(&mut self) -> Result<Option<usize>, DbToolsError>;
    /// Retrieves the next migration step to be run
    fn get_next_step(&self) -> Option<&MigrationStep>;
}

