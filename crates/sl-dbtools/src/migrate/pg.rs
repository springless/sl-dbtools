use sqlx::{query, PgConnection, Row};

use crate::error::DbToolsError;

use super::planner::SchemaVersion;

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
