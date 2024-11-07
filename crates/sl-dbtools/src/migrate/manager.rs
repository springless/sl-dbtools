use std::{fmt::{Display, Write}, io::BufWriter, path::{Path, PathBuf}, str::FromStr};
use chrono::ParseMonthError;
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
    },
    planner::MigrationPlanner,
    step::MigrationPath,
    version::{
        SchemaVersion,
        TargetVersion,
    },
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
}

pub struct PgMigrationManager {
    pub planner: MigrationPlanner,
    pub conn_opts: PgConnectOptions,
    pub view_name: String,
    pub target: TargetVersion,
    folder: PathBuf,
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
        Ok(PgMigrationManager {
            planner,
            conn_opts,
            view_name: view_name.to_string(),
            target: TargetVersion::Current(0),
            folder: folder.as_ref().to_owned(),
        })
    }

    pub fn set_target(&mut self, target: TargetVersion) -> Result<(), DbToolsError> {
        // make sure the target exists
        let _found_target = self.planner.get_target(&target)
            .ok_or(
                DbToolsError::TargetNotFound(target.clone())
            )?;
        self.target = target;
        Ok(())
    }
}

impl MigrationManager for PgMigrationManager {
    fn get_version(&self) -> &SchemaVersion {
        self.planner.get_current()
    }

    fn get_summary_str(&self) -> String {
        let current = self.planner.get_current();
        let target = self.planner.get_target(&self.target)
            .expect("Erroneously failed to find target");

        let mut buf = String::new();

        writeln!(&mut buf, "Target: {}", self.target.to_shorthand()).unwrap();

        self.planner.versions.iter().for_each(|this_version| {
            let tgt_symbol = if this_version == target {
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
        let path = self.planner.current_migration_path(&self.target)?;
        let path_len = path.len();

        if path.is_empty() {
            return Ok(None)
        }

        let next = &path[0];
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
        set_version(&mut tx, &self.view_name, &next.version);
        tx.commit().await?;
        conn.close().await?;

        Ok(Some((path_len - 1).max(0)))
    }
}

impl Display for PgMigrationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_summary_str())
    }
}
