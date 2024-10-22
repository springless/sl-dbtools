use sqlx::{migrate::MigrateDatabase, postgres::PgConnectOptions, Pool, Postgres, Row};
use crate::{
    conn::create_pg_conn,
    db::{
        manager::DbManager,
        namer::{
            DbNamingProps,
            MakeNewConnectOpts,
            ToDbId,
        },
    },
};

pub struct PostgresDbManager {
    /// Connection string for the primary database
    url: String,
    version_table: String,
}

impl PostgresDbManager {
    pub async fn connect(&self) -> Result<Pool<Postgres>, sqlx::Error> {
        create_pg_conn(&self.url).await
    }
    pub async fn get_current_version(&self) -> Result<String, Box<dyn std::error::Error>> {
    //async fn get_current_version(&self) -> Result<Option<crate::migration::MigrationVersion>, Box<dyn std::error::Error>> {
        let conn = self.connect().await;
        let conn = match conn {
            Ok(c) => Ok(c),
            // The error type returned from connect is (), which cannot be converted into
            // std::error::Error
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Could not connect to the database")),
        }?;
        let res = sqlx::query(
            &format!(r#"
                SELECT "version"
                FROM "{}"
            "#, self.version_table)
        )
            .fetch_one(&conn)
            .await?;
        let vers: String = res.try_get(0)?;
        Ok(vers)
    }
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

    async fn load_sql_file<P>(&self, p: P) -> Result<(), Box<dyn std::error::Error>>
    where
        P: AsRef<std::path::Path>
    {
        let conn = create_pg_conn(&self.url).await?;
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

impl MakeNewConnectOpts for PgConnectOptions {
    fn make_new_connection_default(&self, name: Option<&str>) -> Self {
        let base = if let Some(name) = self.get_database() {
            name
        } else {
            "".into()
        };
        let new_name = DbNamingProps::new_default(base, name)
            .to_db_id();
        self.clone().database(&new_name)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::test::TEST_ENV;

    #[tokio::test]
    #[ignore] // until we have a functional automated test db
    async fn test_create_drop_postgres_db() {
        let test_env = TEST_ENV::new_from_env();
        let mgr = PostgresDbManager {
            url: test_env.postgres_url.to_owned(),
            version_table: "_schema_version".into(),
        };
        let _ = mgr.create_database().await;
        assert!(Postgres::database_exists(&test_env.postgres_url).await.unwrap());
        let _ = mgr.drop_database().await;
        assert!(!Postgres::database_exists(&test_env.postgres_url).await.unwrap());
    }

    #[tokio::test]
    #[ignore] // until we have a functional automated test db
    async fn test_get_current_version() {
        let test_env = TestEnv::from_env();
        let mgr = PostgresDbManager {
            url: test_env.postgres_url.to_owned(),
            version_table: "sl_migration".into(),
        };
        let vers = mgr.get_current_version()
            .await
            .unwrap();
        assert_eq!(vers, "My Version");
    }

    #[test]
    fn test_make_new_connection_default() {
        let url = "postgres://user:pass@localhost/dbname";
        let conn = PgConnectOptions::from_str(url).unwrap();
        let new_conn = conn.make_new_connection_default(Some("test_name"));
        assert_ne!(
            new_conn.get_database(),
            conn.get_database(),
        );
    }
}
