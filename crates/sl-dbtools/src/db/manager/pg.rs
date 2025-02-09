use sqlx::{postgres::PgConnectOptions, Postgres};
use crate::{db::{managed::{pg::PgManagedDb, ManagedDb, Seed}, url::DbUrl}, util::pg::{check_if_exists, create_owned_database, create_owned_database_from_template}};

use super::ManagerDb;

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
            &self.conn_opts(),
        ).await?;
        Ok(PgManagedDb::new(url.clone(), Some(self.url.clone()))?)
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
        Ok(PgManagedDb::new(url.clone(), Some(self.url.clone()))?)
    }

     async fn exists(&self, url: &DbUrl) -> Result<bool, sqlx::Error> {
        check_if_exists(self.conn_opts(), url.dbname()).await
    }

     async fn ensure(&self, url: &DbUrl) -> Result<PgManagedDb, sqlx::Error> {
        let exists = self.exists(url).await?;
        if !exists {
            self.create(url).await
        } else {
            self.create(url).await
        }
    }
}

pub enum Initial {
    Empty,
    Template(DbUrl),
}

/// A struct used to simplify the creation of a
pub struct PgManagedDbBuilder {
    base_url: DbUrl,
    admin_url: DbUrl,
    name: Option<String>,
    initial: Initial,
    seeds: Vec<Seed>,
}

impl PgManagedDbBuilder {
    pub fn new(
        base_url: &DbUrl,
        admin_url: &Option<DbUrl>,
        initial: Initial,
    ) -> Result<Self, sqlx::Error> {
        let base_url = base_url.clone();
        // validates that the URL can be parsed to a postgres url
        let _ = base_url.get_pg_conn_opts()?;
        let admin_url = match admin_url {
            Some(url) => url.clone(),
            None => base_url.guess_pg_maintenance_url(),
        };
        // validates that the URL can be parsed to a postgres url
        let _ = admin_url.get_pg_conn_opts()?;
        Ok(PgManagedDbBuilder {
            base_url,
            admin_url,
            name: None,
            initial,
            seeds: vec![],
        })
    }

    pub fn add_seed(mut self, seed: Seed) -> Self {
        self.seeds.push(seed);
        self
    }

    pub fn set_seeds(mut self, seeds: Vec<Seed>) -> Self {
        self.seeds = seeds;
        self
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub async fn build(self) -> Result<PgManagedDb, sqlx::Error> {
        let manager = PgManagerDb::new(self.admin_url)?;

        let managed_url = self.base_url.new_default_transient_url(self.name.as_deref());

        let managed = if let Initial::Template(template_url) = self.initial {
            manager.create_from_template(&managed_url, &template_url).await
        } else {
            manager.create(&managed_url).await
        }?;

        for seed in self.seeds {
            let _ = managed.seed(seed).await?;
        }

        Ok(managed)
    }
}
