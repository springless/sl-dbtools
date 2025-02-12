use std::path::PathBuf;

use sqlx::Database;

/// A seed is used to send SQL to a database and represents the source used to get the
/// script, for example loading from a file or directly from a string in memory.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Seed {
    Sql(String),
    File(PathBuf),
}

impl AsRef<Seed> for Seed {
    fn as_ref(&self) -> &Seed {
        self
    }
}

impl Seed {
    pub async fn raw_sql(&self) -> std::io::Result<String> {
        Ok(match &self {
            Seed::Sql(raw_sql) => raw_sql.to_string(),
            Seed::File(fname) => {
                tokio::fs::read_to_string(&fname)
                    .await?
            },
        })
    }
}

pub trait ManagedDb<D: Database> {
    /// Run a single seed file
    #[allow(async_fn_in_trait)]
    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error>;

    /// Run all provided seeds within a single transaction
    #[allow(async_fn_in_trait)]
    async fn seed_all<S: AsRef<Seed>, I: IntoIterator<Item=S>>(&self, seeds: I) -> Result<(), sqlx::Error>;

    /// Destroy the database
    #[allow(async_fn_in_trait)]
    async fn drop(self) -> Result<(), sqlx::Error>;
}
