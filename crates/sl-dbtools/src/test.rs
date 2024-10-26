use std::sync::LazyLock;

use crate::db::transient::{
    Initial,
    TransientDbBuilder,
    pg::{
        PgTransientDb,
        PgTransientDbBuilder,
    },
};

/// Utility functions for managing test databases

pub fn setup_env() {
    dotenv::from_path("./.test.env").ok();
}

pub struct TestEnv {
    pub postgres_url: String,
    pub postgres_admin_url: Option<String>,
    pub sqlite_url: String,
}

impl TestEnv {
    fn new_from_env() -> Self {
        setup_env();
        TestEnv {
            postgres_url:
                std::env::var("POSTGRES_URL").expect("Set POSTGRES_URL in the environment"),
            postgres_admin_url:
                std::env::var("POSTGRES_ADMIN_URL").ok(),
            sqlite_url:
                std::env::var("SQLITE_URL").expect("Set SQLITE_URL in the environment"),
        }
    }

    pub async fn new_pg_db(&self, test_name: &str, initial: Initial) -> PgTransientDb {
        let transient_db_builder = PgTransientDbBuilder::new(
            &self.postgres_url,
            self.postgres_admin_url.as_deref(),
        )
            .expect("Failed to create transient db builder");
        transient_db_builder.spawn_db(Some(test_name), initial).await
            .expect("Failed to create transient db")
    }
}

pub static TEST_ENV: LazyLock<TestEnv> = LazyLock::new(|| {
    TestEnv::new_from_env()
});

pub fn get_testenv() {

}

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
}
