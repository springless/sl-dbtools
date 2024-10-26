use std::path::PathBuf;

use sqlx::{Connection, Database};

pub mod pg;

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

