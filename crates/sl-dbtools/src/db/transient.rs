use std::path::PathBuf;

use sqlx::PgConnection;

pub enum Initial {
    Empty,
    Template(String),
}

pub enum Seed {
    Sql(String),
    File(PathBuf),
}

pub trait TransientDbBuilder<T: TransientDb> {
    /// Creates a new transient database with the provided options
    #[allow(async_fn_in_trait)]
    async fn spawn_db(&self, base: &str, name: Option<&str>, initial: Initial) -> Result<T, sqlx::Error>;
    /// Returns all of the known transient databases that were spawned with the provided
    /// `base`, and optionally `name`. Used primarily to clean up hanging transient databases.
    #[allow(async_fn_in_trait)]
    async fn find_all(base: &str, name: Option<&str>) -> Result<Vec<T>, sqlx::Error>;
}

pub trait TransientDb {
    #[allow(async_fn_in_trait)]
    async fn drop(self) -> Result<(), sqlx::Error>;
    #[allow(async_fn_in_trait)]
    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error>;
    fn url(&self) -> &str;
}

pub struct PgTransientDbBuilder {

}

pub struct PgTransientDb {
    url: String,
    admin_url: Option<String>,
}

impl TransientDbBuilder<PgTransientDb> for PgTransientDbBuilder {
    async fn spawn_db(
         &self,
         base: &str,
         name: Option<&str>,
         initial: Initial,
    ) -> Result<PgTransientDb, sqlx::Error> {
        // create database
        Ok(PgTransientDb {
            url: base.to_owned(),
            admin_url: None,
        })
    }

    async fn find_all(base: &str, name: Option<&str>) -> Result<Vec<PgTransientDb>, sqlx::Error> {
        Ok(vec![])
    }
}


impl TransientDb for PgTransientDb {
    async fn drop(self) -> Result<(), sqlx::Error> {
        Ok(())
    }

    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error> {
         Ok(())
    }

    fn url(&self) -> &str {
        "Hello"
    }
}
