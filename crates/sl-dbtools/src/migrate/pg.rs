use sqlx::{query, PgConnection, Row};

use crate::error::DbToolsError;

use super::version::SchemaVersion;

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
