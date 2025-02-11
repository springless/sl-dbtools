use clap::Args;
use log::info;

use crate::{
    db::pg::{
        migrate::PgMigrationManager,
        util::file::FileMigrator,
    },
    migrate::{
        manager::MigrationManager,
        version::TargetVersion,
    }
};

use super::{SlArgs, error::CliError};

/// Manages the migration status of the database, including running migrations, checking
/// the current migration version, dry-running migrations, etc.
#[derive(Args, Debug, Clone)]
pub struct MigrateArgs {
    /// The target migration. `HEAD` represents the last version in the migration path,
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

    /// The directory in which all of the migration files are held. Migrations are raw SQL
    /// files that represent an up or down migration of the schema. They should be named
    /// according to whether they are an up or down migration with the template:
    /// `version.up.sql` or `version.dn.sql`, and also named in such a way that sorting
    /// them in lexicographical order puts them in the the order that they should be
    /// run.
    ///
    /// The location of the directory can also be provided in the `MIGRATION_DIR` environment
    /// variable. This flag takes precedence over the environment.
    #[arg(short, long)]
    pub dir: Option<String>,

    /// The name of the view that tracks migrations. Whenever a migration is applied, this
    /// view is updated to return the version that the schema is currently on.
    ///
    /// The name of this view can also be provided by the `MIGRATION_VIEW_NAME` environment
    /// variable. This flag takes precedence over the environment.
    ///
    /// If this value is not provided in a flag or an environment variable it defaults to
    /// `_schema_version`
    #[arg(short, long)]
    pub view_name: Option<String>,

    /// Prints out the sequence of migration files that would run with the specified target
    #[arg(short = 'D', long)]
    pub dry_run: bool,

    /// Under normal circumstances you will be told what is going to happen and asked to
    /// confirm prior to any changes being made to the database. This will prevent those
    /// confirmations.
    #[arg(short, long)]
    pub yes: bool,

    /// Run all of the migrations within the same transaction. Without this flag each version
    /// will be run in its own migration, meaning that in the event of an error, the database
    /// schema version will be the last successful migration. With this flag set, an error
    /// will revert the database to the same state it was in before running any migrations.
    #[arg(short, long)]
    pub all_or_nothing: bool,

    /// Will not run any migrations, but will instead set the current schema version to
    /// be whatever version is being targeted.
    #[arg(short = 'o', long = "override")]
    pub override_version: bool,

    /// Migrate in interactive mode (future)
    #[arg(short, long)]
    pub interactive: bool,

    /// PROPOSED:
    /// Postgres-focused flag which indicates that the migration directory is actually
    /// comprised of multiple subdirectories, each of which handle the migrations for
    /// that specific schema. For example, `migrationdir/public` will run migrations on
    /// the `public` schema and `migrationdir/example` will run migrations on the
    /// `example` schema. The migration files themselves are not treated any differently,
    /// but if you workflow is designed around keeping each schema as a separate entity,
    /// migrated independently, then this lets you specify which schema is being migrated.
    #[arg(short, long)]
    pub schema_dir: bool,

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

    /// Changes the operation of the migration to act on the specified schema file instead
    /// of the current database (although the current database connection is still necessary
    /// as the base for a temporary database that will be created to facilitate the migration).
    /// This will load the provided schema file into a temporary database, run the migrations
    /// on it, and then dump it back on top of the old schema file, replacing it. This is
    /// intended to be used to maintain a copy of the database schema in your codebase.
    /// If this flag is provided along with `--file` flags, then the files passed in those
    /// flags will be assumed to be data-only, and this schema will be loaded prior to each
    /// and before running the migrations. This file will be the last thing migrated at the
    /// end of the process. Only one schema file can be provided.
    #[arg(short='S', long)]
    pub schema_file: Option<String>,
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
            let migrator = FileMigrator {
                base_url: args.get_url()?,
                admin_url: args.get_admin_url()?,
                migration_dir: self.get_migration_dir()?,
                view_name: self.get_view_name(),
            };

            match &self.schema_file {
                Some(schema_file) => {
                    let res = migrator.migrate_files_with_schema(
                        &target,
                        schema_file,
                        migrate_files,
                    ).await?;
                    Ok(res)
                },
                None => {
                    let res = migrator.migrate_files(
                        &target,
                        migrate_files,
                    ).await?;
                    Ok(res)
                },
            }
        } else {
            Err(CliError::MissingArg("No target provided for file migration".to_owned()).into())
        }
    }

    /// Migration being performed on the live database
    async fn migrate_db(&self, args: &SlArgs) -> anyhow::Result<()> {
        let mut manager = PgMigrationManager::new(
            &self.get_migration_dir()?,
            args.get_url()?,
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
