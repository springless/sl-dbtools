use std::sync::LazyLock;

use sqlx::{Database, Pool, Postgres};

/// Utility functions for managing test databases

pub fn setup_env() {
    dotenv::from_path("./.test.env").ok();
}

pub struct TestEnv {
    pub postgres_url: String,
    pub sqlite_url: String,
}

pub trait TestDb {
    fn new_from_env(name: &str, initial: Initial) -> Self;
}


pub struct PgTestDb {
    url: String,
    conn: Option<Pool<Postgres>>,
}

impl PgTestDb {
    pub fn cleanup(&self) {
        
    }
}

impl TestEnv {
    fn new_from_env() -> Self {
        setup_env();
        TestEnv {
            postgres_url:
                std::env::var("POSTGRES_URL").expect("Set POSTGRES_URL in the environment"),
            sqlite_url:
                std::env::var("SQLITE_URL").expect("Set SQLITE_URL in the environment"),
        }
    }

    fn new_pg_db(&self) -> PgTestDb {

    }
}

pub static TEST_ENV: LazyLock<TestEnv> = LazyLock::new(|| {
    TestEnv::from_env()
});

pub fn get_testenv() {

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_env() {
        let pg_url = &SINGLETON_TEST_ENV.postgres_url;
        expec
        assert_eq!(
            pg_url,
            std::env::var("POSTGRES_URL").unwrap(),
        );
    }
}
