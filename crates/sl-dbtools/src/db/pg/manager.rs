use sqlx::{ConnectOptions, Postgres, Row, postgres::PgConnectOptions};
use crate::{
    db::pg::{
        managed::PgManagedDb,
        util::create::{check_if_exists, create_owned_database, create_owned_database_from_template},
    },
    manager::ManagerDb,
    url::DbUrl
};

pub struct PgManagerDb {
    url: DbUrl,
    conn_opts: PgConnectOptions,
}

impl PgManagerDb {
    pub fn new(url: DbUrl) -> Result<Self, sqlx::Error> {
        Ok(PgManagerDb {
            conn_opts: url.get_pg_conn_opts()?,
            url,
        })
    }

    /// Create a new database using another database as a template
    pub async fn create_from_template(&self, url: &DbUrl, template_db: &DbUrl) -> Result<PgManagedDb, sqlx::Error> {
        create_owned_database_from_template(
            &url.get_pg_conn_opts()?,
            &template_db.get_pg_conn_opts()?,
            self.conn_opts(),
        ).await?;
        PgManagedDb::new(url.clone(), Some(self.url.clone()))
    }

    pub fn conn_opts(&self) -> &PgConnectOptions {
        &self.conn_opts
    }
}

impl ManagerDb<Postgres, PgManagedDb> for PgManagerDb {
     async fn create(&self, url: &DbUrl) -> Result<PgManagedDb, sqlx::Error> {
        let _ = create_owned_database(
            &url.get_pg_conn_opts()?,
            self.conn_opts(),
        ).await;
        PgManagedDb::new(url.clone(), Some(self.url.clone()))
    }

     async fn exists(&self, url: &DbUrl) -> Result<bool, sqlx::Error> {
        check_if_exists(self.conn_opts(), url.dbname()).await
    }

     async fn ensure(&self, url: &DbUrl) -> Result<PgManagedDb, sqlx::Error> {
        let exists = self.exists(url).await?;
        if !exists {
            self.create(url).await
        } else {
            PgManagedDb::new(url.clone(), Some(self.url.clone()))
        }
    }

    async fn find_by_regex(&self, regex: &str) -> Result<Vec<PgManagedDb>, sqlx::Error> {
        let mut conn = self.conn_opts().connect().await?;
        let rows = sqlx::query("SELECT datname FROM pg_database WHERE datname ~ $1")
            .bind(regex)
            .fetch_all(&mut conn)
            .await?;
        rows.iter()
            .map(|row| {
                let datname: String = row.try_get("datname")?;
                let mut url = self.url.clone();
                url.set_dbname(&datname);
                PgManagedDb::new(url, Some(self.url.clone()))
            })
            .collect()
    }
}
