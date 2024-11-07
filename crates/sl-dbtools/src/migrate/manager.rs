use std::{fmt::{Display, Write}, path::{Path, PathBuf}, str::FromStr};
use sqlx::{
    Connection,
    postgres::PgConnectOptions,
    PgConnection,
};

use crate::error::DbToolsError;

use super::{
    pg::{
        get_version,
        set_version,
    }, planner::MigrationPlanner, step::{MigrationPath, MigrationStep}, version::{
        SchemaVersion,
        TargetVersion,
    }
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

pub struct PgMigrationManager {
    pub planner: MigrationPlanner,
    pub conn_opts: PgConnectOptions,
    pub view_name: String,
    pub target_version: SchemaVersion,
    migration_path: MigrationPath,
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
        let planner = MigrationPlanner::new_from_folder(&folder, current_version)?;
        let target_version = planner.get_current().clone();
        let migration_path = vec![];
        Ok(PgMigrationManager {
            planner,
            conn_opts,
            view_name: view_name.to_string(),
            target_version,
            migration_path,
        })
    }

    pub fn set_target(&mut self, target: TargetVersion) -> Result<(), DbToolsError> {
        // make sure the target exists
        let found_target = self.planner.get_target(&target)
            .ok_or(
                DbToolsError::TargetNotFound(target.clone())
            )?;
        self.target_version = found_target.clone();
        self.migration_path =
            self.planner.current_migration_path_to_version(&self.target_version)?;
        // we are using this path one at a time, meaning it is most efficient to pop
        // from the end, instead of removing from the front.
        self.migration_path.reverse();
        Ok(())
    }

    pub fn get_full_path(&self) -> &MigrationPath {
        &self.migration_path
    }
}

impl MigrationManager for PgMigrationManager {
    fn get_version(&self) -> &SchemaVersion {
        self.planner.get_current()
    }

    fn get_summary_str(&self) -> String {
        let current = self.planner.get_current();

        let mut buf = String::new();

        writeln!(&mut buf, "Target: {}", self.target_version).unwrap();

        self.planner.versions.iter().for_each(|this_version| {
            let tgt_symbol = if this_version == &self.target_version {
                "->"
            } else {
                "  "
            };
            let at_symbol = if this_version == current {
                "@ "
            } else {
                "  "
            };

            let version_text = if let SchemaVersion::Version(version_str) = this_version {
                &version_str
            } else {
                "(ROOT)"
            };
            writeln!(&mut buf, "{}{}{}",
                tgt_symbol,
                at_symbol,
                version_text,
            ).unwrap();
        });

        writeln!(&mut buf, "    ^HEAD^").unwrap();
        buf
    }

    /// Attempts to run the next migration in the given migration path. If the `Ok` value of
    /// the result is `Some` it will contain the number of migrations remaining until
    /// the path is complete, if it is `None` then that means that it has finished the
    /// final migration and does not need to do anything else.
    ///
    /// NOTE: Convert this to an iterator at some point.
    async fn do_next_migration(&mut self) -> Result<Option<usize>, DbToolsError> {
        let path_len = self.migration_path.len();

        let next = &self.migration_path.last();
        // we're out of steps
        if &None == next { return Ok(None) }
        let next = next.unwrap();

        let raw_sql = next.action.get_raw_sql().await?;

        let mut conn = PgConnection::connect_with(&self.conn_opts).await?;
        let mut tx = conn.begin().await?;

        // run the migration file, if it exists
        if let Some(raw_sql) = raw_sql {
            sqlx::raw_sql(&raw_sql)
                .execute(&mut *tx)
                .await?;
        }

        // update the version stored in the database before ending the transaction
        set_version(&mut tx, &self.view_name, &next.version).await?;
        tx.commit().await?;
        conn.close().await?;

        // At this point, we should be able to say confidently that the version has changed
        // and update it inside our planner
        self.planner.set_current(&next.version)?;

        // remove the path step we just finished
        self.migration_path.pop();
        Ok(Some((path_len).max(0)))
    }

    fn get_next_step(&self) -> Option<&MigrationStep> {
        self.migration_path.last()
    }
}

impl Display for PgMigrationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_summary_str())
    }
}
