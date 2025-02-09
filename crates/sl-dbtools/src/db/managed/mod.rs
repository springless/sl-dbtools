use std::path::PathBuf;

use sqlx::{Connection, Database};

pub mod pg;

/// A seed is used to send SQL to a database and represents the source used to get the
/// script, for example loading from a file or directly from a string in memory.
pub enum Seed {
    Sql(String),
    File(PathBuf),
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
    #[allow(async_fn_in_trait)]
    async fn seed(&self, seed: Seed) -> Result<(), sqlx::Error>;

    #[allow(async_fn_in_trait)]
    async fn drop(self) -> Result<(), sqlx::Error>;
}
