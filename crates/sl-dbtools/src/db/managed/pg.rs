use sqlx::{postgres::{PgConnectOptions, PgPoolOptions}, Postgres};

use crate::{db::{manager::pg::PgManagerDb, url::DbUrl}, util};

use super::ManagedDb;

pub struct PgManagedDb {
    url: DbUrl,
    conn_opts: PgConnectOptions,
    manager: PgManagerDb,
}

impl PgManagedDb {
    pub fn new(url: DbUrl, manager_url: Option<DbUrl>) -> Result<Self, sqlx::Error> {
        let manager_url = if let Some(url) = manager_url {
            url
        } else {
            url.guess_pg_maintenance_url()
        };
        Ok(PgManagedDb {
            conn_opts: url.get_pg_conn_opts()?,
            url,
            manager: PgManagerDb::new(manager_url)?,
        })
    }

    pub fn url(&self) -> &DbUrl {
        &self.url
    }

    pub fn conn_opts(&self) -> &PgConnectOptions {
        &self.conn_opts
    }
}

impl ManagedDb<Postgres> for PgManagedDb {
    async fn seed(&self, seed: super::Seed) -> Result<(), sqlx::Error> {
        let conn = PgPoolOptions::new()
            .connect_with(self.conn_opts.clone())
            .await?;
        let raw_sql = seed.raw_sql().await?;
        let mut tx = conn.begin().await?;
        sqlx::raw_sql(&raw_sql)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn drop(self) -> Result<(), sqlx::Error> {
        util::pg::force_drop_database(
            &self.conn_opts(),
            &self.manager.conn_opts(),
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::{managed::Seed, manager::pg::Initial}, test::TEST_ENV};
    use sqlx::Row;

    #[tokio::test]
    async fn test_db_create() {
        let testdb = TEST_ENV.new_pg_db(
            "test_db_create",
            Initial::Empty,
            vec![],
        ).await;
        let conn = PgPoolOptions::new()
            .connect_with(testdb.url().get_pg_conn_opts().unwrap())
            .await;
        assert!(matches!(conn, Ok(_)));
        let drop = testdb.drop().await;
        assert!(matches!(drop, Ok(_)));
    }

    #[tokio::test]
    async fn test_db_create_seeded_file() {
        let testdb = TEST_ENV.new_pg_db(
            "test_db_create_seeded_file",
            Initial::Empty,
            vec![
                Seed::File("pg/00-test-seed.sql".into()),
            ],
        ).await;
        let conn = PgPoolOptions::new()
            .connect_with(testdb.url().get_pg_conn_opts().unwrap())
            .await
            .unwrap();
        let row = sqlx::query("SELECT username FROM \"user\" ORDER BY username ASC")
            .fetch_one(&conn)
            .await
            .expect("");

        let name: String = row.try_get("username").expect("");

        assert_eq!(name, "user1");

        let _ = testdb.drop().await;
    }

    #[tokio::test]
    async fn test_db_create_seeded_sql() {
        let testdb = TEST_ENV.new_pg_db(
            "test_db_create_seeded_sql",
            Initial::Empty,
            vec![
                Seed::Sql(r#"
                CREATE TABLE my_table (
                    id SERIAL PRIMARY KEY
                    ,value TEXT NOT NULL
                );
                INSERT INTO my_table (value) VALUES
                ('00-first-value')
                ,('01-second-value')
                ,('02-third-value');
                "#.to_string()),
            ],
        ).await;

        let conn = PgPoolOptions::new()
            .connect_with(testdb.url().get_pg_conn_opts().unwrap())
            .await
            .unwrap();
        let row = sqlx::query("SELECT value FROM my_table ORDER BY value ASC")
            .fetch_one(&conn)
            .await
            .expect("");

        let val: String = row.try_get("value").expect("");

        assert_eq!(val, "00-first-value");

        let _ = testdb.drop().await;
    }

    #[tokio::test]
    async fn test_db_create_template() {
        let template_testdb = TEST_ENV.new_pg_db(
            "test_db_create_template",
            Initial::Empty,
            vec![
                Seed::Sql(r#"
                CREATE TABLE my_table (
                    id SERIAL PRIMARY KEY
                    ,value TEXT NOT NULL
                );
                INSERT INTO my_table (value) VALUES
                ('00-first-value')
                ,('01-second-value')
                ,('02-third-value');
                "#.to_string()),
            ],
        ).await;
        let created_testdb = TEST_ENV.new_pg_db(
            "test_db_create_template_created",
            Initial::Template(template_testdb.url().clone()),
            vec![],
        ).await;

        let conn = PgPoolOptions::new()
            .connect_with(created_testdb.url().get_pg_conn_opts().unwrap())
            .await
            .unwrap();
        let row = sqlx::query("SELECT value FROM my_table ORDER BY value ASC")
            .fetch_one(&conn)
            .await
            .expect("");

        let val: String = row.try_get("value").expect("");

        assert_eq!(val, "00-first-value");

        let _ = template_testdb.drop().await;
        let _ = created_testdb.drop().await;
    }
}
