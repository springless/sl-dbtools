use clap::Args;

use crate::dump::pgdump::dump_db;

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
        dump_db(&args.get_url()?, &self.file, self.with_data, &self.schema)?;
        Ok(())
    }
}
