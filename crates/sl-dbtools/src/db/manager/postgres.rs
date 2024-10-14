use sqlx::{migrate::MigrateDatabase, Postgres};
use crate::db::manager::DbManager;

pub struct PostgresDbManager {
    /// Connection string for the primary database
    url: String,
}

impl DbManager for PostgresDbManager {
    async fn create_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let created_db = Postgres::create_database(&self.url)
            .await?;
        Ok(created_db)
    }

    async fn drop_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dropped_db = Postgres::drop_database(&self.url)
            .await?;
        Ok(dropped_db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::TestEnv;

    #[tokio::test]
    async fn test_create_drop_postgres_db() {
        let test_env = TestEnv::from_env();
        let mgr = PostgresDbManager { url: test_env.postgres_url.to_owned() };
        let _ = mgr.create_database().await;
        assert!(Postgres::database_exists(&test_env.postgres_url).await.unwrap());
        let _ = mgr.drop_database().await;
        assert!(!Postgres::database_exists(&test_env.postgres_url).await.unwrap());
    }
}
