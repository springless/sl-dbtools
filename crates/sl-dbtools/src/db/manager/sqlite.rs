use sqlx::{migrate::MigrateDatabase, Sqlite};
use crate::{conn::create_sqlite_conn, db::manager::DbManager};

pub struct SqliteDbManager {
    url: String,
}

impl DbManager for SqliteDbManager {
    async fn create_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let created_db = Sqlite::create_database(&self.url)
            .await?;
        Ok(created_db)
    }
    async fn drop_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dropped_db = Sqlite::drop_database(&self.url)
            .await?;
        Ok(dropped_db)
    }
    async fn load_sql_file<P>(&self, p: P) -> Result<(), Box<dyn std::error::Error>>
    where
        P: AsRef<std::path::Path>
    {
        let conn = create_sqlite_conn(&self.url).await?;
        let raw_sql = tokio::fs::read_to_string(p.as_ref())
            .await?;
        let mut tx = conn.begin().await?;
        sqlx::query(&raw_sql)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::TestEnv;

    #[tokio::test]
    async fn test_create_drop_sqlite_db() {
        let test_env = TestEnv::from_env();
        let mgr = SqliteDbManager { url: test_env.sqlite_url.to_owned() };
        let _ = mgr.create_database().await;
        assert!(Sqlite::database_exists(&test_env.sqlite_url).await.unwrap());
        let _ = mgr.drop_database().await;
        assert!(!Sqlite::database_exists(&test_env.sqlite_url).await.unwrap());
    }
}
