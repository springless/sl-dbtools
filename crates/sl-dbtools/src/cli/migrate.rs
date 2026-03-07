use std::path::PathBuf;

use clap::Args;
use log::info;

use crate::{
    db::pg::{
        managed::PgManagedDb,
        manager::PgManagerDb,
        migrate::PgMigrationManager,
        util::file::FileMigrator,
    },
    managed::{
        ManagedDb,
        Seed,
    },
    migrate::{
        manager::MigrationManager,
        version::TargetVersion,
    },
    manager::ManagerDb,
};

use super::{SlArgs, error::CliError};

/// Manage migration status of the database
///
/// Manages the migration status of the database, including running migrations, checking
/// the current migration version, dry-running migrations, etc.
#[derive(Args, Debug, Clone)]
pub struct MigrateArgs {
    /// The target migration
    ///
    /// `HEAD` represents the last version in the migration path,
    /// while `@` references the current database version. For example, given versions `v01..v10`,
    /// if the database is currently on `v05`, a target of `HEAD` will run migrations `v06..v10`.
    ///
    /// This also supports relative targets using tilde (`~`) for referencing previous versions.
    /// For instance, `HEAD~2` refers to "2 versions before HEAD" (i.e., `v08`), and `@~3` will
    /// downgrade three versions (`v02` in our example).
    ///
    /// To refer to future versions from the current one, use `+`, so `@+2` will run the next
    /// two migrations (upgrading to `v07` in our example). `HEAD+n` is obviously meaningless.
    ///
    /// To avoid naming conflicts with versions that contain `+` or `~`, relative targets are
    /// restricted to `HEAD` and `@`.
    ///
    /// For targeting a specific version, you may type either all or part of the migration version
    /// as long as the part uniquely identifies it. For instance, in the `v01..v10` path, a target
    /// of `7` will uniquely identify `v07`, while a target of `v0` is ambiguous.
    ///
    /// If this value is not provided then `migrate` will print the current schema version and
    /// exit.
    #[arg(value_name="TARGET", index=1)]
    pub target: Option<String>,

    /// The directory that holds the migration files
    ///
    /// Migrations are raw SQL files that represent an up or down migration of the schema.
    /// They should be named according to whether they are an up or down migration with the
    /// template: `version.up.sql` or `version.dn.sql`, and also named in such a way that
    /// sorting them in lexicographical order puts them in the the order that they should be run.
    ///
    /// The location of the directory can also be provided in the `MIGRATION_DIR` environment
    /// variable. This flag takes precedence over the environment.
    #[arg(short, long)]
    pub dir: Option<String>,

    /// The name of the view that tracks migration version
    ///
    /// Whenever a migration is applied, this view is updated to return the version that
    /// the schema is currently on.
    ///
    /// The name of this view can also be provided by the `MIGRATION_VIEW_NAME` environment
    /// variable. This flag takes precedence over the environment.
    ///
    /// If this value is not provided in a flag or an environment variable it defaults to
    /// `_schema_version`
    #[arg(short, long)]
    pub view_name: Option<String>,

    /// Only print the sequence of migrations that will be run for the target
    #[arg(short = 'D', long)]
    pub dry_run: bool,

    /// Auto-approve any prompts
    ///
    /// Under normal circumstances you will be told what is going to happen and asked to
    /// confirm prior to any changes being made to the database. This will prevent those
    /// confirmations.
    #[arg(short, long)]
    pub yes: bool,

    /// Run all migrations in a single transaction
    ///
    /// Without this flag each version will be run in its own migration, meaning that
    /// in the event of an error, the database schema version will be the last successful
    /// migration. With this flag set, an error will revert the database to the same state
    /// it was in before running any migrations.
    #[arg(short, long)]
    pub all_or_nothing: bool,

    /// Force-set the schema version in the database
    ///
    /// Will not run any migrations, but will instead set the current schema version to
    /// be whatever version is being targeted.
    #[arg(short = 'o', long = "override")]
    pub override_version: bool,

    /// Migrate in interactive mode (future)
    #[arg(short, long)]
    pub interactive: bool,

    /// (Proposed) Specify a directory of files that comprise the schema
    ///
    /// PROPOSED, NOT CURRENTLY IMPLEMENTED:
    /// Postgres-focused flag which indicates that the migration directory is actually
    /// comprised of multiple subdirectories, each of which handle the migrations for
    /// that specific schema. For example, `migrationdir/public` will run migrations on
    /// the `public` schema and `migrationdir/example` will run migrations on the
    /// `example` schema. The migration files themselves are not treated any differently,
    /// but if you workflow is designed around keeping each schema as a separate entity,
    /// migrated independently, then this lets you specify which schema is being migrated.
    #[arg(short, long)]
    pub schema_dir: bool,

    /// Run the migration on a file
    ///
    /// Changes the operation of the migration to act on a specified file or directory instead
    /// of the current database (although the current database connection is still necessary
    /// as the base for temporary databases that will be created to facilitate the migration).
    /// This will read in each SQL file in the directory (or the singular SQL file specified),
    /// run the migrations on it to the specified target, and then dump it back out on top
    /// of the old file, replacing it. This is intended to be used in seed and fixture databases
    /// to quickly apply migrations to test data without having to manually load and dump
    /// those fixtures. If the `--schema-file` flag is passed, then each of the fixtures
    /// will be assumed to **not** include a schema, and so they will be loaded and dumped
    /// as data-only, using the provided Schema File as the base schema. At the end of the
    /// migrations, the schema file will also be migrated and dumped per the description of
    /// that flag. Multiple directories or files can be passed in successive `-f` flags.
    #[arg(short, long)]
    pub file: Option<Vec<String>>,

    /// Run the migration on a schema-only file
    ///
    /// Changes the operation of the migration to act on the specified schema file instead
    /// of the current database (although the current database connection is still necessary
    /// as the base for a temporary database that will be created to facilitate the migration).
    /// This will load the provided schema file into a temporary database, run the migrations
    /// on it, and then dump it back on top of the old schema file, replacing it. This is
    /// intended to be used to maintain a copy of the database schema in your codebase.
    /// If this flag is provided along with `--file` flags, then the files passed in those
    /// flags will be assumed to be data-only, and this schema will be loaded prior to each
    /// and before running the migrations. This file will be the last thing migrated at the
    /// end of the process. Only one schema file can be provided. If the filname passed is
    /// `HEAD`, then it will locate or operate on a `HEAD.sql` file located in the migration
    /// path.
    #[arg(short='S', long)]
    pub schema_file: Option<String>,

    /// Drop and create the database
    ///
    /// This will cause the database to be dropped prior to running migrations (meaning
    /// it will always only run `up` migrations). It will also run a `ROOT.sql` file in
    /// the migration folder -- if it exists -- prior to running the rest of the migrations.
    #[arg(short='R', long)]
    pub remake: bool,

    /// Create the database if it does not exist
    ///
    /// If the database already exists then will take no action, but if the database does
    /// not yet exist then it will create it and run a `ROOT.sql` file in the migration folder,
    /// -- if it exists -- prior to running the rest of the migrations.
    #[arg(short, long)]
    pub ensure: bool,
}

const ENV_MIGRATION_DIR: &str = "MIGRATION_DIR";
const ENV_MIGRATION_VIEW_NAME: &str = "MIGRATION_VIEW_NAME";
const DEFAULT_VIEW_NAME: &str = "_schema_version";

impl MigrateArgs {
    pub fn get_dir(&self) -> Option<String> {
        match &self.dir {
            Some(dir) => Some(dir.to_owned()),
            None => std::env::var(ENV_MIGRATION_DIR).ok(),
        }
    }

    pub fn get_view_name(&self) -> String {
        match &self.view_name {
            Some(view_name) => view_name.to_owned(),
            None => {
                std::env::var(ENV_MIGRATION_VIEW_NAME)
                    .unwrap_or(DEFAULT_VIEW_NAME.to_string())
            },
        }
    }

    fn print_config(&self) {
        info!("Dir: {}", self.get_dir().unwrap_or("NONE".to_owned()));
        info!("View name: {}", self.get_view_name());
    }

    fn get_migration_dir(&self) -> Result<String, CliError> {
        let url = if let Some(url) = &self.dir {
            url
        } else {
            &std::env::var(ENV_MIGRATION_DIR).ok().ok_or(
                CliError::MissingArg(format!("Provide --dir or {}", ENV_MIGRATION_DIR))
            )?
        };
        Ok(url.clone())
    }

    /// Migration being performed on one or more files
    async fn migrate_files(&self, args: &SlArgs) -> anyhow::Result<()> {
        let migrate_files = if let Some(files) = &self.file {
            files.clone()
        } else {
            vec![]
        };

        if let Some(target) = &self.target {
            let target = TargetVersion::new_from_str(&target);

            let base_url = args.get_url()?;
            let admin_url = args.get_admin_url()?;
            let migration_dir = self.get_migration_dir()?;
            let view_name = self.get_view_name();

            match &self.schema_file {
                Some(schema_file_passed) => {
                    let schema_file = if schema_file_passed == "HEAD" {
                        let candidate = PathBuf::from(&migration_dir).join("HEAD.sql");
                        if !candidate.exists() {
                            // no `HEAD.sql` file exists, so we will either copy the `ROOT.sql`
                            // file into its place if that exists, or make an empty file if
                            // `ROOT.sql` does not exist to ensure the database has a
                            // starting point.
                            let root_seed_file = {
                                let candidate = PathBuf::from(self.get_migration_dir()?).join("ROOT.sql");
                                candidate.exists().then_some(candidate)
                            };
                            if let Some(fname) = root_seed_file {
                                std::fs::copy(&fname, &candidate)?;
                            } else {
                                std::fs::File::create(&candidate)?;
                            }
                        }
                        candidate
                    } else {
                        PathBuf::from(schema_file_passed)
                    };
                    FileMigrator::migrate_files_with_schema(
                        base_url,
                        Some(admin_url),
                        &migration_dir,
                        &view_name,
                        &target,
                        schema_file,
                        migrate_files,
                    ).await?;
                    Ok(())
                },
                None => {
                    FileMigrator::migrate_files(
                        base_url,
                        Some(admin_url),
                        &migration_dir,
                        &view_name,
                        &target,
                        migrate_files,
                    ).await?;
                    Ok(())
                },
            }
        } else {
            Err(CliError::MissingArg("No target provided for file migration".to_owned()).into())
        }
    }

    /// Migration being performed on the live database
    async fn migrate_db(&self, args: &SlArgs) -> anyhow::Result<()> {
        let db_url = args.get_url()?;

        if self.ensure || self.remake {
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
            // if `ROOT.sql` exists in the migrations folder, run that prior to the
            // rest of the migrations if we have just created the database
            let root_seed_file = {
                let candidate = PathBuf::from(self.get_migration_dir()?).join("ROOT.sql");
                candidate.exists().then_some(candidate)
            };
            if let Some(fname) = root_seed_file {
                let seed = Seed::File(fname);
                managed.seed(seed).await?;
            }
        }

        let mut manager = PgMigrationManager::new(
            &self.get_migration_dir()?,
            db_url,
            &self.get_view_name(),
        ).await?;


        if let Some(target_str) = &self.target {
            let target = TargetVersion::new_from_str(target_str);
            manager.set_target(target.clone())?;

            info!("Initial state:");
            info!("{}", manager);

            if None == manager.get_next_step() {
                info!("Already at target: {}", &target);
                return Ok(());
            }

            while let Some(next_step) = manager.get_next_step() {
                info!(
                    "Migrating: {} -> {}",
                    &manager.planner.get_current(),
                    next_step.version,
                );
                manager.do_next_migration().await?;
            }
            info!("");
            info!("Done migrating. Final state:");
            info!("{}", manager);
        } else {
            info!("{}", manager);
        }

        Ok(())
    }

    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        if args.verbose {
            self.print_config();
        }

        if let Some(_) = self.schema_file {
            self.migrate_files(args).await
        } else if let Some(_) = self.file {
            self.migrate_files(args).await
        } else {
            self.migrate_db(args).await
        }
    }
}
