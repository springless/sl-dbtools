use thiserror::Error;

use crate::error::DbToolsError;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Missing argument: {0}")]
    MissingArg(String),
    #[error(transparent)]
    DbToolsError(#[from] DbToolsError),
}
