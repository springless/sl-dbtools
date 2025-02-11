use std::{fmt::{Display, Write}, path::Path};
use sqlx::{
    query, Connection, PgConnection, Row,
};

use crate::{
    url::DbUrl,
    error::{
        DbToolsError,
        SqlErrorWithContext,
    },
    migrate::{
        manager::MigrationManager,
        planner::MigrationPlanner,
        step::{MigrationPath, MigrationStep},
        version::{
            SchemaVersion,
            TargetVersion,
        },
    },
};

pub struct PgMigrationManager {
    pub planner: MigrationPlanner,
    pub url: DbUrl,
    pub view_name: String,
    pub target_version: SchemaVersion,
    migration_path: MigrationPath,
}

impl PgMigrationManager {
    pub async fn new<P>(
        folder: P,
        url: DbUrl,
        view_name: &str,
    ) -> Result<Self, DbToolsError>
    where P: AsRef<Path>
    {
        let mut conn = PgConnection::connect_with(&url.get_pg_conn_opts()?).await?;
        let current_version = get_version(&mut conn, view_name).await?;
        let planner = MigrationPlanner::new_from_folder(&folder, current_version)?;
        let target_version = planner.get_current().clone();
        let migration_path = vec![];
        Ok(PgMigrationManager {
            planner,
            url,
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

        let mut conn = PgConnection::connect_with(&self.url.get_pg_conn_opts()?).await?;
        let mut tx = conn.begin().await?;

        // run the migration file, if it exists
        if let Some(raw_sql) = raw_sql {
            let res = sqlx::raw_sql(&raw_sql)
                .execute(&mut *tx)
                .await;
            if let Err(sqlx_err) = res {
                return Err(DbToolsError::SqlWithContext(SqlErrorWithContext {
                    e: sqlx_err,
                    filename: None,
                    query: Some(raw_sql),
                }))
            }
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


pub async fn get_version(conn: &mut PgConnection, view_name: &str) -> Result<SchemaVersion, DbToolsError> {
    let row = query(&format!("SELECT version FROM {}", view_name))
        .fetch_one(conn)
        .await;

    let version = match row {
        Ok(r) => {
            if let Ok(version) = r.try_get("version") {
                Ok(SchemaVersion::Version(version))
            } else {
                // I think this shouldn't be possible, but we'll accept it as
                // ROOT for now, anyways
                Ok(SchemaVersion::Root)
            }
        },
        Err(e) => match e {
            sqlx::Error::Database(_) => {
                // if the database reports that the view does not exist, then that means
                // that we're at the `ROOT` version. For the time being we're just taking
                // any database error to mean that the view did not exist
                Ok(SchemaVersion::Root)
            },
            _ => Err(e),
        }
    };
    Ok(version?)
}

pub async fn set_version(conn: &mut PgConnection, view_name: &str, version: &SchemaVersion) -> Result<(), DbToolsError> {
    let q = match version {
        SchemaVersion::Root => {
            format!("DROP VIEW IF EXISTS {}", view_name)
        },
        SchemaVersion::Version(ver_str) => {
            format!(
                "CREATE OR REPLACE VIEW {} AS SELECT '{}'::TEXT AS version",
                view_name,
                ver_str,
            )
        },
    };

    sqlx::query(&q)
        .execute(conn)
        .await?;

    Ok(())
}
