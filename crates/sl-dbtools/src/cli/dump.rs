use std::fs::File;

use clap::Args;

use crate::db::pg::util::dump::{dump_db, DumpType};

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
}

impl DumpArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        let dump_type = if self.dump_schema && !self.dump_data {
            DumpType::SchemaOnly
        } else if self.dump_data && !self.dump_schema {
            DumpType::DataOnly
        } else { DumpType::All };

        let mut fwriter = File::create(&self.file)?;
        dump_db(&args.get_url()?, &mut fwriter, &dump_type, &self.schema)?;
        Ok(())
    }
}
