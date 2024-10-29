use clap::{Parser, Subcommand};
use migrate::MigrateArgs;
use temp::TempArgs;

mod migrate;
mod temp;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct SlArgs {
    /// URL of the main database. Can also be provided by `DATABASE_URL` in the environment. This
    /// flag will take precedence over the environment variable. This should conform to the
    /// typical URL database specification: `protocol://user:pass@host:port/database`
    #[arg(short, long)]
    pub url: Option<String>,

    /// URL of the administrative database, required only for database creation or deletion
    /// actions under certain conditions. If provided, this URL will be used to connect as an
    /// admin for creating and dropping databases. This value can also be provided by the
    /// `DATABASE_ADMIN_URL` environment variable. This flag will take precedence over the
    /// environment variable.
    ///
    /// By default, the system will attempt to use the main database credentials and connect
    /// to `postgres` as the maintenance database. You must provide an admin url if:
    ///
    /// 1. You are creating or dropping a database, AND
    /// 2. Either:
    ///     a. The `postgres` database is not available as a maintenance database, OR
    ///     b. The main database credentials lack permissions to create or drop databases.
    #[arg(short, long)]
    pub admin_url: Option<String>,

    /// Reads in the `.env` file provided prior to running. The database and admin database
    /// URLs can be provided in the environment using `DATABASE_URL` and `DATABASE_ADMIN_URL`,
    /// instead of passing them as command line arguments. You can pass in multiple `.env`
    /// files to read by setting this flag multiple times and it will read them all in the
    /// order provided. By default this will try to read in `.env` in the current directory.
    /// If any `-e` argument is provided it will instead read only the proveded `.env` files,
    /// or if `--no-env` is provided it will read in none of them (even if `-e` is set)
    #[arg(short, long)]
    env: Option<Vec<String>>,

    /// Does not read any `.env` files prior to running the program. By default, it will
    /// attempt to read `.env` in the current directory. Setting this flag will prevent the
    /// program from doing that, as well as prevent it from reading any explicitly-passed
    /// `.env` files from `-e` flags.
    #[arg(short = 'E', long)]
    no_env: bool,

    #[command(subcommand)]
    command: SlSubcommand,
}

#[derive(Subcommand, Debug, Clone)]
enum SlSubcommand {
    Migrate(MigrateArgs),
    Temp(TempArgs),
}

impl SlSubcommand {
    pub fn run(&self, args: &SlArgs) {
        match self {
            Self::Migrate(sub_args) => {
                sub_args.run(args);
            },
            Self::Temp(sub_args) => {
                sub_args.run(args);
            },
        }
    }
}

impl SlArgs {
    pub fn run(&self) {
        // attempt to read a `.env` file unless explicitly told not to
        if !self.no_env {
            if let Some(env_files) = &self.env {
                // If we were passed a `-e` then only read in the ones passed
                env_files.into_iter().for_each(|fname| { dotenv::from_path(fname).ok(); });
            } else {
                // otherwise attempt to read the standard `.env` file
                dotenv::dotenv().ok();
            }
        }
        let _ = &self.command.run(self);
    }
}

