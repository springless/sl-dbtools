use std::{
    path::{Path, PathBuf},
};
use super::version::SchemaVersion;
use crate::error::DbToolsError;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Direction {
    Up,
    Dn,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum MigrationAction {
    /// We are intentionally and explicitly not taking an action
    NoAction,
    /// An action was potentially possible, but the file for that action was not found.
    /// While this might be an error, it is assumed that the file was not meant to be there.
    /// This is separate from `NoAction` for the sake of being explicit as well as
    /// troubleshooting.
    NoFileFound,
    /// A file was found with the enclosed path which should be run as part of this action.
    /// The file is not greedily loaded, so has to be loaded manually from the path.
    SqlFile(PathBuf),
}

/// A specific step in a migration path, representing an up or down migration
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct MigrationStep {
    /// The version the schema will be in after performing the action
    pub version: SchemaVersion,
    /// The action to perform for this step of the migration
    pub action: MigrationAction,
}

impl MigrationAction {
    /// Given a version and direction attempts to find the SQL file associated with that
    /// migration. If it finds one, it will return a `SqlFile` action, and if not then
    /// a `NoAction` migration.
    pub fn new_from_direction<P: AsRef<Path>>(
        direction: &Direction,
        version: &SchemaVersion,
        folder: P,
    ) -> Self {
        match version {
            SchemaVersion::Root => Self::NoAction,
            SchemaVersion::Version(vers_name) => {
                let fname = format!(
                    "{}.{}.sql",
                    vers_name,
                    if &Direction::Up == direction { "up" } else { "dn" },
                );
                let full_path = folder.as_ref().join(fname);
                if full_path.exists() {
                    Self::SqlFile(full_path)
                } else { Self::NoFileFound }
            },
        }
    }

    /// Will fetch the raw SQL for this action. In the event of a file being loaded, this is
    /// performed asynchronously, which is why this function returns a promise. For any
    /// no-op action this will return `None`
    pub async fn get_raw_sql(&self) -> Result<Option<String>, DbToolsError> {
        match self {
            MigrationAction::SqlFile(fpath) => {
                let raw_sql = tokio::fs::read_to_string(fpath).await?;
                Ok(Some(raw_sql))
            },
            _ => Ok(None),
        }
    }
}

pub type MigrationPath = Vec<MigrationStep>;

