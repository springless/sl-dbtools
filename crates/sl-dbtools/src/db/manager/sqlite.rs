use sqlx::{migrate::MigrateDatabase, Sqlite};
use crate::db::manager::DbManager;

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
