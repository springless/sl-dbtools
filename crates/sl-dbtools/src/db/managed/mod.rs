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

pub trait ManagedDbBuilder<D: Database, T: ManagedDb<D>> {
    // Add a seed to run after creating the database. If this is called multiple times,
    // it should run multiple seeds, in the order provided.
    fn add_seed(self, seed: Seed) -> Self;
    fn set_seeds(self, seeds: Vec<Seed>) -> Self;
    /// Sets the addon name of the database
    fn set_name(self, name: String) -> Self;
    /// Creates a new managed database with the provided options
    #[allow(async_fn_in_trait)]
    async fn build(self) -> Result<T, sqlx::Error>;
    /// Returns all of the known managed databases that were spawned with the provided
    /// `base`, and optionally `name`. Used primarily to clean up hanging managed databases.
    #[allow(async_fn_in_trait)]
    async fn find_all(base: &str, name: Option<&str>) -> Result<Vec<T>, sqlx::Error>;
}

pub trait ManagedDb<D: Database> {
    #[allow(async_fn_in_trait)]
    async fn drop(self) -> Result<(), sqlx::Error>;
    #[allow(async_fn_in_trait)]
    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error>;
    fn conn_opts(&self) -> &<D::Connection as Connection>::Options;
}

