use clap::Args;
use log::info;
use crate::{
    cli::error::CliError,
    db::pg::{
        managed::PgManagedDb,
        manager::PgManagerDb,
    },
    managed::{
        ManagedDb,
        Seed,
    },
    manager::ManagerDb,
    namer::{DbNamingOpts, DbNamingTemplate},
};

use super::SlArgs;

/// Load files into a database
///
/// This can also be used to create a new database with the specified seeds.
#[derive(Args, Debug, Clone)]
pub struct LoadArgs {
    /// The files to load
    ///
    /// Files will be loaded in the order provided
    #[arg(value_name="SEED")]
    pub seed: Vec<String>,

    /// Create new temporary database
    ///
    /// If this is set then it will load into a new temporary database with a name
    /// that is derived from the main database URL and a timestamp
    #[arg(short, long)]
    pub temporary: bool,

    /// Drop and create the database
    ///
    /// If this is set then it will destroy the database if it currently exists and then
    /// remake it, passing in the provided seed files
    #[arg(short, long)]
    pub remake: bool,

    /// Create the database if it does not exist
    ///
    /// If the database already exists then it will run the passed in files on the existing
    /// database, but if it does not then the database will be created prior to running the
    /// files. If this is not provided then it will error in the event that the database
    /// is missing.
    #[arg(short, long)]
    pub ensure: bool,

    /// Run all seeds in a sigle transaction
    ///
    /// If this is not set then each seed will run in a separate transaction until one
    /// fails, and all subsequent seeds will be skipped
    #[arg(short='A', long)]
    pub all_or_nothing: bool,
}

impl LoadArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        let db_url = args.get_url()?;
        let db_url = if self.temporary {
            db_url.new_temp_url(DbNamingOpts {
                base: None,
                name: None,
                pattern: DbNamingTemplate::Pattern("z{timestamp}_load_{uuid}".into()),
                keep_full: false,
            })
        } else { db_url };

        if self.seed.is_empty() {
            return Err(CliError::InvalidArg("No seed files passed".into()))?;
        }

        info!("Loading into database: {}", db_url.to_string());

        let manager_url = args.get_admin_url()?;

        let managed_db = if self.ensure || self.remake {
            let manager_url = args.get_admin_url()?;
            let manager = PgManagerDb::new(manager_url.clone())?;
            if self.remake {
                info!("Remake set; dropping database");
                // first destroy the database
                let managed = PgManagedDb::new(db_url.clone(), Some(manager_url.clone()))?;
                managed.drop().await?;
                info!("...Dropped");
            }
            info!("Ensuring database exists");
            // Now make sure it exists
            let managed = manager.ensure(&db_url).await?;
            managed
        } else {
            info!("Not ensuring database");
            // We're just going to create the managed database and hope for the
            // best
            PgManagedDb::new(db_url.clone(), Some(manager_url.clone()))?
        };

        info!("Seeding database");

        let seed_iter = self.seed.iter().map(|s| Seed::File(s.into()));

        // At this point the database should exist and `managed_db` should point to it
        if self.all_or_nothing {
            info!("Running all seeds");
            managed_db.seed_all(seed_iter).await?;
        } else {
            for seed in seed_iter {
                match &seed {
                    Seed::File(fname) => { info!("Seeding from file: {}", fname.to_string_lossy()); },
                    Seed::Sql(_) => { info!("Running raw sql..."); },
                }
                managed_db.seed(seed).await?;
            }
        }

        Ok(())
    }
}
