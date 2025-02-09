use std::path::Path;

use crate::db::{
    managed::{
        pg::PgManagedDb,
        Seed,
    }, manager::pg::{
        Initial, PgManagedDbBuilder
    }, url::DbUrl
};

/// Convenience wrapper for a struct that can quickly create managed
/// databases on request for use in unit testing. It is recommended
/// to create a singleton instance of this test environment in a common
/// testing file, and then utilize it in each test. eg.
///
/// ```rust,ignore
/// // in common file
/// use sl_dbtools::{
///     testing::pg::PgTestEnv,
///     db::managed::{Initial, Seed},
/// };
/// use std::sync::LazyLock;
/// pub static TEST_ENV: LazyLock<PgTestEnv> = LazyLock::new(|| {
///     PgTestEnv::new_from_env()
/// });
/// ```
/// ```rust,ignore
/// // in testing file
/// #[cfg(test)]
/// mod tests {
///     use sl_dbtools::db::managed::{Initial, Seed};
///     use crate::common::test::TEST_ENV;
///
///     #[tokio::test]
///     async fn test_my_app() {
///         // create the database
///         let testdb = TEST_ENV.new_pg_db(
///             "test_my_app", // <- extra name field
///             Initial::Empty, // <- initial database state
///             vec![], // <- seeds
///         )
///             .await.expect("");
///         // connect to the database
///         let conn = PgPoolOptions::new()
///             .connect_with(testdb.url.clone())
///             .await
///             .unwrap();
///         // ... do tests
///         // drop the database
///         let _ = testdb.drop().await;
///     }
/// }
/// ```
pub struct PgTestEnv {
    pub base_url: DbUrl,
    pub admin_url: Option<DbUrl>,
}

impl PgTestEnv {
    /// Creates a new `PgTestEnv` based on values pulled from the environment.
    /// It will look for `DATABASE_URL` to find the base URL for new managed
    /// databases, as well as `DATABASE_ADMIN_URL` to find the URL to use to
    /// connect to the postgres server with permissions to generate a new database.
    /// If `DATABASE_ADMIN_URL` is not provided, it will attempt to use the same
    /// credentials of `DATABASE_URL` to connect to the `postgres` or `template1`
    /// database. If a path to a `.env` file is passed to this function it will
    /// attempt to load it with `dotenv` prior to checking the environment
    /// variables.
    pub fn new_from_env_file<P: AsRef<Path>>(env_file: Option<P>) -> Self {
        if let Some(path) = env_file {
            dotenv::from_path(path.as_ref()).ok();
        }
        PgTestEnv {
            base_url: DbUrl::parse(
                &std::env::var("DATABASE_URL").expect("Set DATABASE_URL in the environment")
            ).expect("DATABASE_URL is invalid"),
            admin_url: std::env::var("DATABASE_ADMIN_URL")
                .ok()
                .map(|url| DbUrl::parse(&url).expect("DATABASE_ADMIN_URL is provided but invalid")),
        }
    }

    /// Creates a new PgTestEnv like `new_from_env_file` but passing `.env` as the default
    /// environment file
    pub fn new_from_env() -> Self {
        Self::new_from_env_file(Some("./.env"))
    }

    pub async fn new_pg_db(
        &self,
        test_name: &str,
        initial: Initial,
        seeds: Vec<Seed>,
    ) -> PgManagedDb {
        PgManagedDbBuilder::new(
            &self.base_url,
            &self.admin_url,
            initial,
        )
            .expect("Failed to create managed db builder")
            .set_name(test_name.to_owned())
            .set_seeds(seeds)
            .build()
            .await
            .expect("Failed to create managed db")
    }
}

