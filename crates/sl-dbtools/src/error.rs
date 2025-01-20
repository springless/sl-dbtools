use std::io;
use sqlx::postgres::{PgDatabaseError, PgErrorPosition};
use thiserror::Error;

use crate::{migrate::version::{SchemaVersion, TargetVersion}, util::formatting::get_query_pos_str};

#[derive(Error, Debug)]
pub enum DbToolsError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Process \"{}\" failed with status: {:?}", .0, .1)]
    ProcessStatus(String, Option<i32>),
    /// An error returned when attempting to set a concrete version that does not exist, such as
    /// when setting the `current` version of a MigrationPlanner when that version does not actually
    /// exist within the plan.
    #[error("Invalid schema version: {0}")]
    VersionDoesNotExist(SchemaVersion),
    /// An error returned when a command is issued or when attempting to build
    /// a migration path while targeting a version that cannot be found
    #[error("Unable to find target: {0}")]
    TargetNotFound(TargetVersion),
    /// An error returned by the SQL engine
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    /// An error returned by the SQL engine with extra query context for troubleshooting
    /// purposes
    #[error("{}", .0)]
    SqlWithContext(SqlErrorWithContext),
}

pub struct SqlErrorWithContext {
    pub e: sqlx::Error,
    pub filename: Option<String>,
    pub query: Option<String>,
}

impl std::fmt::Debug for SqlErrorWithContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        Ok(())
    }
}

/// The default error output from sqlx does not include the detail/hint information
/// from Postgresql, so we take control of the error output to include that additional
/// context.
impl std::fmt::Display for SqlErrorWithContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.e {
            sqlx::Error::Database(db_error) => {
                if let Some(pg_error) = db_error.try_downcast_ref::<PgDatabaseError>() {
                    self.e.fmt(f)?;

                    if let Some(detail) = pg_error.detail() {
                        write!(f, "\nDetail: {}", detail)?;
                    }
                    if let Some(hint) = pg_error.hint() {
                        write!(f, "\nHint: {}", hint)?;
                    }
                    if let Some(fname) = &self.filename {
                        write!(f, "\nIn file: {}", fname)?;
                    }
                    if let Some(err_pos) = pg_error.position() {
                        match err_pos {
                            PgErrorPosition::Original(idx) => {
                                // we have to use the passed in query to figure out where
                                // the actual error is
                                write!(f, "\nAt character: {}", idx)?;
                                if let Some(query) = &self.query {
                                    write!(f, "\nIn query:\n{}", get_query_pos_str(query, idx))?;
                                } else {
                                    write!(f, "\nQuery unknown")?;
                                }
                            },
                            PgErrorPosition::Internal { position, query } => {
                                write!(
                                    f,
                                    "\nAt character: {}\nIn query:\n{}",
                                    position,
                                    query,
                                )?;
                            },
                        }
                    }
                    Ok(())
                } else {
                    self.e.fmt(f)
                }
            },
            _ => self.e.fmt(f),
        }
    }
}
