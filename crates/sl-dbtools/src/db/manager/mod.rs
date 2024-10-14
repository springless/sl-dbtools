/// Module for managing the creation and deletion of databases

pub trait DbManager {
    /// Attempts to create the database described by this manager. Will error if that database
    /// cannot be created or already exists.
    async fn create_database(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn drop_database(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait MigrationManager {
    async fn do_migration(&self) -> Result<(), Box<dyn std::error::Error>>;
}

pub mod postgres;
pub mod sqlite;
