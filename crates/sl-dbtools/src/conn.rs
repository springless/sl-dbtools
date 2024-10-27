use sqlx::{postgres::PgPoolOptions, sqlite::SqlitePoolOptions, Pool, Postgres, Sqlite};

/// Utility functions for creating and managing database connections

pub async fn create_pg_conn(conn_str: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(conn_str)
        .await
}

pub async fn create_sqlite_conn(conn_str: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(conn_str)
        .await
}
