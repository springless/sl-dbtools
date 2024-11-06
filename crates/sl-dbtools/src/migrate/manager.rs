use std::{io::BufWriter, path::Path, str::FromStr, fmt::Write};
use chrono::ParseMonthError;
use sqlx::{
    Connection,
    postgres::PgConnectOptions,
    PgConnection,
};

use crate::error::DbToolsError;

use super::{pg::get_version, planner::{MigrationPath, MigrationPlanner, SchemaVersion, TargetVersion}};

pub trait MigrationManager {
    /// Retrieves the current schema version from the database
    fn get_version(&self) -> &SchemaVersion;
    /// Returns a string representing a cli-compatible printout of the current migration
    /// status.
    fn get_summary_str(&self) -> String;
    /// Performs the specified migration. It should do this within the context of a single
    /// transaction, which rolls back if there is an error.
    #[allow(async_fn_in_trait)]
    async fn do_next_migration(&mut self) -> Result<(), DbToolsError>;
}

pub struct PgMigrationManager {
    pub planner: MigrationPlanner,
    pub conn_opts: PgConnectOptions,
    pub view_name: String,
    pub target: TargetVersion,
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
            target: TargetVersion::Current(0),
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
        let full_path = self.planner
            .build_migration_path(&TargetVersion::Root(0), &TargetVersion::Head(0))
            // It should be impossible to fail this.
            .expect("Erroneously failed to build ROOT -> HEAD migration path");
        let current = self.planner.get_current();
        let target = self.planner.get_target(&self.target)
            .expect("Erroneously failed to find target");

        let mut buf = String::new();

        writeln!(&mut buf, "Target: {}", self.target.to_shorthand()).unwrap();

        match full_path {
            MigrationPath::Up(versions) => {
                versions.iter().for_each(|this_version| {
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
                })
            },
            _ => writeln!(&mut buf, "No versions available").unwrap(),
        }
        writeln!(&mut buf, "    ^HEAD^").unwrap();
        buf
    }

    async fn do_next_migration(&mut self) -> Result<(), DbToolsError> {
        Ok(())
    }
}
