use sqlx::{query, PgConnection, Row};

use crate::error::DbToolsError;

use super::version::SchemaVersion;

pub async fn get_version(conn: &mut PgConnection, view_name: &str) -> Result<SchemaVersion, DbToolsError> {
    let row = query(&format!("SELECT version FROM {}", view_name))
        .fetch_one(conn)
        .await?;
    let version_res: Result<String, _> = row.try_get("version");
    let version = match version_res {
        Ok(vers) => SchemaVersion::Version(vers),
        Err(_) => SchemaVersion::Root,
    };
    Ok(version)
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
