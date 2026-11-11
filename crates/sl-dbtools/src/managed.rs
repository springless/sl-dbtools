use std::path::PathBuf;

use sqlx::Database;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::db::{DataSection, line_section};

/// A seed is used to send SQL to a database and represents the source used to get the
/// script, for example loading from a file or directly from a string in memory.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Seed {
    /// A raw SQL string, passed directly
    Sql(String),
    /// A filename for a file to be read in full
    File(PathBuf),
    /// A filename for a file that has sections, where only one section should be read
    FileSection(PathBuf, DataSection),
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
            Seed::FileSection(fname, section) => {
                let f = tokio::fs::File::open(fname).await?;
                let mut lines = BufReader::new(f).lines();

                // skip until we reach the section marker
                loop {
                    match lines.next_line().await? {
                        Some(thisl) if line_section(&thisl) == Some(*section) => break,
                        Some(_) => continue,
                        None => return Ok("".to_string()),
                    }
                }

                // collect lines until the next marker (or the end of the file)
                let mut out = String::new();
                while let Some(line) = lines.next_line().await? {
                    match line_section(&line) {
                        Some(found_section) if found_section == *section => continue,
                        Some(_) => break,
                        None => {
                            out.push_str(&line);
                            out.push('\n');
                        }
                    }
                }

                out
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
