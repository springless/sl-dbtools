/// Utility functions for managing test databases

pub fn setup_env() {
    dotenv::from_path("./.test.env").ok();
}

pub struct TestEnv {
    pub postgres_url: String,
    pub sqlite_url: String,
}

impl TestEnv {
    pub fn from_env() -> Self {
        setup_env();
        TestEnv {
            postgres_url:
                std::env::var("POSTGRES_URL").expect("Set POSTGRES_URL in the environment"),
            sqlite_url:
                std::env::var("SQLITE_URL").expect("Set SQLITE_URL in the environment"),
        }
    }
}

