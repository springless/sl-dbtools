use std::path::Path;

use crate::migration::MigrationVersion;

/// Module for managing the creation and deletion of databases

pub trait DbManager {
    /// Attempts to create the database described by this manager. Will error if that database
    /// cannot be created or already exists.
    #[allow(async_fn_in_trait)]
    async fn create_database(&self) -> Result<(), Box<dyn std::error::Error>>;
    #[allow(async_fn_in_trait)]
    async fn drop_database(&self) -> Result<(), Box<dyn std::error::Error>>;
    #[allow(async_fn_in_trait)]
    async fn load_sql_file<P>(&self, p: P) -> Result<(), Box<dyn std::error::Error>>
        where
            P: AsRef<Path>;
//    async fn get_current_version(&self) -> Result<Option<MigrationVersion>, Box<dyn std::error::Error>>;
}

pub trait MigrationManager {
    #[allow(async_fn_in_trait)]
    async fn do_migration(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub mod postgres;
pub mod sqlite;

