use clap::Args;

use crate::dump::pgdump::dump_db;

use super::SlArgs;

#[derive(Args, Debug, Clone)]
pub struct DumpArgs {
    /// The file to which the database should be dumped
    #[arg(short, long)]
    pub file: String,
}

impl DumpArgs {
    pub async fn run(&self, args: &SlArgs) -> anyhow::Result<()> {
        dump_db(&args.get_url()?, &self.file, false)?;
        Ok(())
    }
}
