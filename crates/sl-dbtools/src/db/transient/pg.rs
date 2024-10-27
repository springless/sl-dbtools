use std::{path::PathBuf, str::FromStr};

use sqlx::{postgres::{PgConnectOptions, PgPoolOptions}, Connection, Database, Postgres};
use crate::{
    db::namer::{MakeNewConnectOpts, DbNamingProps, ToDbId},
    util,
};
use super::{TransientDb, TransientDbBuilder, Initial, Seed};

pub struct PgTransientDbBuilder {
    base_url: PgConnectOptions,
    admin_url: PgConnectOptions,
    name: Option<String>,
    initial: Initial,
    seeds: Vec<Seed>,
}

pub struct PgTransientDb {
    pub url: PgConnectOptions,
    admin_url: PgConnectOptions,
}

impl PgTransientDbBuilder {
    pub fn new(
        base_url: &str,
        admin_url: Option<&str>,
        initial: Initial,
    ) -> Result<Self, sqlx::Error> {
        let base_opts = PgConnectOptions::from_str(base_url)?;
        let admin_opts = match admin_url {
            Some(url) => PgConnectOptions::from_str(url)?,
            None => util::pg::parse_for_maintenance(&base_opts),
        };
        Ok(PgTransientDbBuilder {
            base_url: base_opts,
            admin_url: admin_opts,
            name: None,
            initial,
            seeds: vec![],
        })
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

impl TransientDbBuilder<Postgres, PgTransientDb> for PgTransientDbBuilder {
    fn add_seed(mut self, seed: Seed) -> Self {
        self.seeds.push(seed);
        self
    }

    fn set_seeds(mut self, seeds: Vec<Seed>) -> Self {
        self.seeds = seeds;
        self
    }

    fn set_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    async fn build(
         self,
    ) -> Result<PgTransientDb, sqlx::Error> {
        // create database
        let transient_conn_opts = self.base_url
            .make_new_connection_default(self.name.as_deref());

        let _db_res = match &self.initial {
            Initial::Empty => {
                util::pg::create_owned_database(
                    &transient_conn_opts,
                    &self.admin_url,
                )
                    .await?
            },
            Initial::Template(template_url) => {
                util::pg::create_owned_database_from_template(
                    &transient_conn_opts,
                    &PgConnectOptions::from_str(&template_url)?,
                    &self.admin_url,
                )
                    .await?
            },
        };

        let transient_db = PgTransientDb {
            url: transient_conn_opts,
            admin_url: self.admin_url.to_owned(),
        };

        for seed in self.seeds {
            let _ = transient_db.seed(seed).await?;
        }

        Ok(transient_db)
    }

    async fn find_all(base: &str, name: Option<&str>) -> Result<Vec<PgTransientDb>, sqlx::Error> {
        Ok(vec![])
    }
}


impl TransientDb<Postgres> for PgTransientDb {
    async fn drop(self) -> Result<(), sqlx::Error> {
        util::pg::force_drop_database(
            &self.url,
            &self.admin_url,
        ).await
    }

    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error> {
        let conn = PgPoolOptions::new()
            .connect_with(self.url.clone())
            .await?;
        let raw_sql = match seed {
            Seed::Sql(raw_sql) => {
                raw_sql
            },
            Seed::File(fname) => {
                tokio::fs::read_to_string(&fname)
                    .await?
            },
        };
        let mut tx = conn.begin().await?;
        sqlx::raw_sql(&raw_sql)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    fn conn_opts(&self) -> &PgConnectOptions {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::TEST_ENV;
    use sqlx::{
        Row,
        ConnectOptions,
    };

    #[tokio::test]
    async fn test_db_create() {
        let testdb = TEST_ENV.new_pg_db(
            "test_db_create",
            Initial::Empty,
            vec![],
        ).await;
        let conn = PgPoolOptions::new()
            .connect_with(testdb.url.clone())
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
            .connect_with(testdb.url.clone())
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
            .connect_with(testdb.url.clone())
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
            Initial::Template(template_testdb.url.to_url_lossy().to_string()),
            vec![],
        ).await;

        let conn = PgPoolOptions::new()
            .connect_with(created_testdb.url.clone())
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
