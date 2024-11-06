use std::{path::Path, str::FromStr};
use chrono::ParseMonthError;
use sqlx::{
    Connection,
    postgres::PgConnectOptions,
    PgConnection,
};

use crate::error::DbToolsError;

use super::{pg::get_version, planner::{MigrationPlanner, SchemaVersion, TargetVersion}};

pub trait MigrationManager {
    /// Retrieves the current schema version from the database
    fn get_version(&self) -> &SchemaVersion;
    /// Returns a string representing a cli-compatible printout of the current migration
    /// status.
    fn get_summary_str(&self) -> String;
    async fn do_migration(&mut self, target: TargetVersion) -> Result<(), DbToolsError>;
}

pub struct PgMigrationManager {
    pub planner: MigrationPlanner,
    pub conn_opts: PgConnectOptions,
    pub view_name: String,
}

impl PgMigrationManager {
    pub async fn new<P>(
        folder: P,
        url: &str,
        view_name: &str,
    ) -> Result<Self, DbToolsError>
    where P: AsRef<Path>
    {
        let conn_opts = PgConnectOptions::from_str(url)?;
        let mut conn = PgConnection::connect_with(&conn_opts).await?;
        let current_version = get_version(&mut conn, view_name).await?;
        let planner = MigrationPlanner::new_from_folder(folder, current_version)?;
        Ok(PgMigrationManager {
            planner,
            conn_opts,
            view_name: view_name.to_string(),
        })
    }
}

impl MigrationManager for PgMigrationManager {
    fn get_version(&self) -> &SchemaVersion {
        self.planner.get_current()
    }

    fn get_summary_str(&self) -> String {
        "hello".to_string()
    }

    async fn do_migration(&mut self, target: TargetVersion) -> Result<(), DbToolsError> {
        Ok(())
    }
}
