use clap::Args;

use super::SlArgs;

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
}

const ENV_MIGRATION_DIR: &str = "MIGRATION_DIR";
const ENV_MIGRATION_VIEW_NAME: &str = "MIGRATION_VIEW_NAME";

impl MigrateArgs {
    pub fn get_dir(&self) -> Option<String> {
        match &self.dir {
            Some(dir) => Some(dir.to_owned()),
            None => std::env::var(ENV_MIGRATION_DIR).ok(),
        }
    }

    pub fn get_view_name(&self) -> Option<String> {
        match &self.view_name {
            Some(view_name) => Some(view_name.to_owned()),
            None => std::env::var(ENV_MIGRATION_VIEW_NAME).ok(),
        }
    }

    fn print_config(&self) {
        println!("Dir: {}", self.get_dir().unwrap_or("NONE".to_owned()));
        println!("View name: {}", self.get_view_name().unwrap_or("NONE".to_owned()));
    }

    pub fn run(&self, args: &SlArgs) {
        if !args.quiet {
            self.print_config();
        }
        //let migration_path = MigrationPath::new_from_folder
        println!("Migrate {:?}", self.target);
    }
}
