use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

/// Utility functions for creating and managing database connections

pub async fn create_pg_conn(conn_str: &str) -> Result<Pool<Postgres>, ()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(conn_str)
        .await;
    match pool {
        Ok(pool) => Ok(pool),
        Err(_) => Err(())
    }
}

