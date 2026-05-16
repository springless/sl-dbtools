use crate::{
    db::pg::{managed::PgManagedDb, manager::PgManagerDb}, managed::{ManagedDb, Seed}, manager::ManagerDb, namer::{DbNamingOpts, DbNamingTemplate}, url::DbUrl
};

pub enum Initial {
    Empty,
    Template(DbUrl),
}

/// A struct used to simplify the creation of a temporary database, effectively
/// a database intended to be created, modified, and then destroyed in the process
/// of running migrations or tests.
pub struct PgTempDbBuilder {
    base_url: DbUrl,
    admin_url: DbUrl,
    name: Option<String>,
    initial: Initial,
    seeds: Vec<Seed>,
    pattern: DbNamingTemplate,
}

impl PgTempDbBuilder {
    pub fn new(
        base_url: &DbUrl,
        admin_url: &Option<DbUrl>,
        initial: Initial,
        pattern: DbNamingTemplate,
    ) -> Result<Self, sqlx::Error> {
        let base_url = base_url.clone();
        // validates that the URL can be parsed to a postgres url
        let _ = base_url.get_pg_conn_opts()?;
        let admin_url = match admin_url {
            Some(url) => url.clone(),
            None => base_url.guess_pg_maintenance_url(),
        };
        // validate that the URL can be parsed to a postgres url
        let _ = admin_url.get_pg_conn_opts()?;
        Ok(PgTempDbBuilder {
            base_url,
            admin_url,
            name: None,
            initial,
            seeds: vec![],
            pattern,
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

        let managed_url = self.base_url.new_temp_url(DbNamingOpts {
            pattern: self.pattern,
            base: None,
            name: self.name,
            keep_full: false,
        });

        let managed = if let Initial::Template(template_url) = self.initial {
            manager.create_from_template(&managed_url, &template_url).await
        } else {
            manager.create(&managed_url).await
        }?;

        for seed in self.seeds {
            managed.seed(seed).await?;
        }

        Ok(managed)
    }
}
