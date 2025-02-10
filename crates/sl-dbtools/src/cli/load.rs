use clap::Args;
use log::info;
use super::SlArgs;

/// Loads provided seed files into a database. This can also be used to create a new
/// database with the specified seeds.
#[derive(Args, Debug, Clone)]
pub struct LoadArgs {
    /// The file to load. Multiple files can be passed and they will
    /// be loaded in the order provided
    #[arg(value_name="SEED")]
    pub seed: Vec<String>,
    /// If this is set then it will load into a new database with a name that is
    /// derived from the main database URL
    #[arg(short, long)]
    pub new: bool,
    /// If this is set then it will destroy the database if it currently exists and then
    /// remake it with, passing in the provided seed files
    #[arg(short, long)]
    pub remake: bool,
}

impl LoadArgs {
    pub async fn run(&self, _args: &SlArgs) -> anyhow::Result<()> {
        info!("{:?}", self);
        Ok(())
    }
}
