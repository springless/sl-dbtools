use std::fs::File;

use clap::Args;

use crate::dump::pgdump::{dump_db, DumpType};

use super::SlArgs;

/// Dump the current database schema to a specified file, with or without data.
#[derive(Args, Debug, Clone)]
pub struct DumpArgs {
    /// The file to which the database should be dumped
    #[arg(short, long)]
    pub file: String,
    /// Include data in the dump
    #[arg(short='d', long)]
    pub with_data: bool,
    #[arg(short='n', long)]
    pub schema: Option<Vec<String>>,
}

impl DumpArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        let dump_type = if self.with_data {
            DumpType::All
        } else {
            DumpType::SchemaOnly
        };
        let mut fwriter = File::create(&self.file)?;
        dump_db(&args.get_url()?, &mut fwriter, &dump_type, &self.schema)?;
        Ok(())
    }
}
