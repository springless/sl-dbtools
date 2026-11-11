use std::fs::File;

use clap::Args;

use crate::{cli::migrate::{DEFAULT_VIEW_NAME, ENV_MIGRATION_VIEW_NAME}, db::{pg::{migrate::get_version_from_url, util::dump::{DumpType, dump_db}}}, migrate::version::SchemaVersion};

use super::SlArgs;

/// Dump the current database schema to a specified file, with or without data.
#[derive(Args, Debug, Clone)]
pub struct DumpArgs {
    /// The file to which the database should be dumped
    #[arg(short, long)]
    pub file: String,

    /// Only dump the schema
    ///
    /// If this is provided without `dump-data` then only the schema will be dumped, without
    /// any data. If both this an `data-only` are provided, then it will
    /// dump both, which is also the default if neither are provided.
    #[arg(short='s', long)]
    pub dump_schema: bool,

    /// Only dump data
    ///
    /// If this is provided without `dump-schema`, then only the data will be dumped, without
    /// any of the schema information. If both this and `schema-only` are
    /// provided, then it will dump both, which is also the default if
    /// neither are provided.
    #[arg(short='d', long)]
    pub dump_data: bool,

    /// Database schemas from which to dump
    ///
    /// You can specify this multiple times to dump multiple schemas from the
    /// database.
    #[arg(short='n', long)]
    pub schema: Option<Vec<String>>,

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

    /// Include the current version of the schema in the dump
    ///
    /// Adds a comment at the top of the dump describing the current schema version, as
    /// indicated by the configured schema view
    #[arg(short='V', long)]
    pub with_version: bool,
}

impl DumpArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        let dump_type = if self.dump_schema && !self.dump_data {
            DumpType::SchemaOnly
        } else if self.dump_data && !self.dump_schema {
            DumpType::DataOnly
        } else { DumpType::All };

        let mut fwriter = File::create(&self.file)?;

        let dburl = &args.get_url()?;

        let version = if self.with_version {
            get_version_from_url(&dburl, &self.get_view_name()).await?
        } else { SchemaVersion::Root };

        dump_db(dburl, &mut fwriter, &dump_type, &self.schema, &version)?;
        Ok(())
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
}
