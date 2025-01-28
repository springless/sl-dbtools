use std::{path::{Path, PathBuf}, str::FromStr, sync::LazyLock};

use sqlx::postgres::PgConnectOptions;

use crate::{db::transient::{
    pg::{
        PgTransientDb,
        PgTransientDbBuilder,
    }, Initial, Seed, TransientDbBuilder
}, util};

/// Utility functions for managing test databases

pub fn setup_env() {
    dotenv::from_path("./.test.env").ok();
}

pub struct TestEnv {
    pub postgres_url: String,
    postgres_admin_url: Option<String>,
    pub sqlite_url: String,
    seed_dir: PathBuf,
}

impl TestEnv {
    fn new_from_env() -> Self {
        setup_env();
        let seed_dir_str = std::env::var("SEED_DIR").expect("Set SEED_DIR in the environment");
        let seed_dir = Path::new(&seed_dir_str).to_owned();
        if !seed_dir.exists() {
            eprintln!("Warning: provided SEED_DIR path does not exist: {:?}", seed_dir);
        }
        TestEnv {
            postgres_url:
                std::env::var("POSTGRES_URL").expect("Set POSTGRES_URL in the environment"),
            postgres_admin_url:
                std::env::var("POSTGRES_ADMIN_URL").ok(),
            sqlite_url:
                std::env::var("SQLITE_URL").expect("Set SQLITE_URL in the environment"),
            seed_dir,
        }
    }

    pub fn get_postgres_conn(&self) -> PgConnectOptions {
        PgConnectOptions::from_str(&self.postgres_url).unwrap()
    }

    pub fn get_postgres_admin_conn(&self) -> PgConnectOptions {
        match &self.postgres_admin_url {
            Some(url) => PgConnectOptions::from_str(&url).unwrap(),
            None => util::pg::parse_for_maintenance(&self.get_postgres_conn())
        }
    }

    pub async fn new_pg_db(
        &self,
        test_name: &str,
        initial: Initial,
        seeds: Vec<Seed>,
    ) -> PgTransientDb {
        // change any `File` seeds to be relative to the SEED_DIR
        let seeds = seeds.into_iter().map(|seed| {
            if let Seed::File(path) = seed {
                Seed::File(self.seed_path(path))
            } else {
                seed
            }
        })
            .collect();
        PgTransientDbBuilder::new(
            &self.postgres_url,
            self.postgres_admin_url.as_deref(),
            initial,
        )
            .expect("Failed to create transient db builder")
            .set_name(test_name.to_owned())
            .set_seeds(seeds)
            .build()
            .await
            .expect("Failed to create transient db")
    }

    /// Given a `Path`, will append it to the seed path provided in the environment
    pub fn seed_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let p = path.as_ref();
        self.seed_dir.join(p)
    }
}

pub static TEST_ENV: LazyLock<TestEnv> = LazyLock::new(|| {
    TestEnv::new_from_env()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_env() {
        let pg_url = &TEST_ENV.postgres_url;
        assert_eq!(
            pg_url,
            &std::env::var("POSTGRES_URL").unwrap(),
        );
    }

    #[test]
    fn test_test_env_seed_path() {
        let file = "pg/00-test-seed.sql";
        let path = TEST_ENV.seed_path(file);
        assert_eq!(
            path.to_string_lossy(),
            TEST_ENV.seed_dir.join(file).to_string_lossy(),
        );
        assert!(path.exists());
    }
}
