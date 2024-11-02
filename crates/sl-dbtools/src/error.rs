use std::io;
use thiserror::Error;

use crate::migrate::planner::{SchemaVersion, TargetVersion};

#[derive(Error, Debug)]
pub enum DbToolsError {
    #[error(transparent)]
    Io(#[from] io::Error),
    /// An error returned when attempting to set a concrete version that does not exist, such as
    /// when setting the `current` version of a MigrationPlanner when that version does not actually
    /// exist within the plan.
    #[error("Invalid schema version: {0:?}")]
    VersionDoesNotExist(SchemaVersion),
    /// An error returned when a command is issued or when attempting to build
    /// a migration path while targeting a version that cannot be found
    #[error("Unable to find target: {0:?}")]
    TargetNotFound(TargetVersion),
}
