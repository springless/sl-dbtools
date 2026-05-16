use sqlx::Database;

use super::{managed::ManagedDb, url::DbUrl};

/// A ManagerDb is responsible for creating and destroying databases
pub trait ManagerDb<D: Database, T: ManagedDb<D>> {
    /// Create a new database. Will return an error if the database already exists
    #[allow(async_fn_in_trait)]
    async fn create(&self, url: &DbUrl) -> Result<T, sqlx::Error>;

    /// Checks whether a database with a given name exists
    #[allow(async_fn_in_trait)]
    async fn exists(&self, url: &DbUrl) -> Result<bool, sqlx::Error>;

    /// Ensure a database exists. If it does, then this will do nothing and return
    /// the ManagedDb instance. If it does not, then it will create the database
    /// first.
    #[allow(async_fn_in_trait)]
    async fn ensure(&self, url: &DbUrl) -> Result<T, sqlx::Error>;

    /// Finds all databases matching the passed regex and returns them as
    /// managed databases
    #[allow(async_fn_in_trait)]
    async fn find_by_regex(&self, regex: &str) -> Result<Vec<T>, sqlx::Error>;
}
