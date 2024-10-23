use std::{path::PathBuf, str::FromStr};

use sqlx::{postgres::{PgConnectOptions, PgPoolOptions}, Connection, Database, Postgres};
use crate::{
    db::namer::{MakeNewConnectOpts, DbNamingProps, ToDbId},
    util::pg,
};

pub enum Initial {
    Empty,
    Template(String),
}

pub enum Seed {
    Sql(String),
    File(PathBuf),
}

pub trait TransientDbBuilder<D: Database, T: TransientDb<D>> {
    /// Creates a new transient database with the provided options
    #[allow(async_fn_in_trait)]
    async fn spawn_db(&self, name: Option<&str>, initial: Initial) -> Result<T, sqlx::Error>;
    /// Returns all of the known transient databases that were spawned with the provided
    /// `base`, and optionally `name`. Used primarily to clean up hanging transient databases.
    #[allow(async_fn_in_trait)]
    async fn find_all(base: &str, name: Option<&str>) -> Result<Vec<T>, sqlx::Error>;
}

pub trait TransientDb<D: Database> {
    #[allow(async_fn_in_trait)]
    async fn drop(self) -> Result<(), sqlx::Error>;
    #[allow(async_fn_in_trait)]
    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error>;
    fn conn_opts(&self) -> &<D::Connection as Connection>::Options;
}

pub struct PgTransientDbBuilder {
    base_url: PgConnectOptions,
    admin_url: PgConnectOptions,
}

pub struct PgTransientDb {
    url: PgConnectOptions,
    admin_url: PgConnectOptions,
}

impl PgTransientDbBuilder {
    pub fn new(base_url: &str, admin_url: Option<&str>) -> Result<Self, sqlx::Error> {
        let base_opts = PgConnectOptions::from_str(base_url)?;
        let admin_opts = match admin_url {
            Some(url) => PgConnectOptions::from_str(url)?,
            None => pg::parse_for_maintenance(&base_opts),
        };
        Ok(PgTransientDbBuilder {
            base_url: base_opts,
            admin_url: admin_opts,
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
    async fn spawn_db(
         &self,
         name: Option<&str>,
         initial: Initial,
    ) -> Result<PgTransientDb, sqlx::Error> {
        // create database
        let transient_conn_opts = self.base_url
            .make_new_connection_default(name);

        let _db_res = match initial {
            Initial::Empty => {
                pg::create_owned_database(
                    &transient_conn_opts,
                    &self.admin_url,
                )
                    .await?
            },
            Initial::Template(template_url) => {
                pg::create_owned_database_from_template(
                    &transient_conn_opts,
                    &PgConnectOptions::from_str(&template_url)?,
                    &self.admin_url,
                )
                    .await?
            },
        };

        Ok(PgTransientDb {
            url: transient_conn_opts,
            admin_url: self.admin_url.to_owned(),
        })
    }

    async fn find_all(base: &str, name: Option<&str>) -> Result<Vec<PgTransientDb>, sqlx::Error> {
        Ok(vec![])
    }
}


impl TransientDb<Postgres> for PgTransientDb {
    async fn drop(self) -> Result<(), sqlx::Error> {
        pg::force_drop_database(
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
        sqlx::query(&raw_sql)
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

    mod pg {
        use super::*;

        #[tokio::test]
        async fn test_db_create() {
            let testdb = TEST_ENV.new_pg_db(
                "test_db_create",
                Initial::Empty,
            ).await;
            let conn = PgPoolOptions::new()
                .connect_with(testdb.url.clone())
                .await;
            assert!(matches!(conn, Ok(_)));
            let drop = testdb.drop().await;
            assert!(matches!(drop, Ok(_)));
        }
    }
}
