use clap::Args;
use log::info;
use super::SlArgs;

/// Load files into a database
///
/// This can also be used to create a new database with the specified seeds.
#[derive(Args, Debug, Clone)]
pub struct LoadArgs {
    /// The files to load
    ///
    /// Files will be loaded in the order provided
    #[arg(value_name="SEED")]
    pub seed: Vec<String>,

    /// Create new temporary database
    ///
    /// If this is set then it will load into a new temporary database with a name
    /// that is derived from the main database URL and a timestamp
    #[arg(short, long)]
    pub temporary: bool,

    /// Drop and create the database
    ///
    /// If this is set then it will destroy the database if it currently exists and then
    /// remake it, passing in the provided seed files
    #[arg(short, long)]
    pub remake: bool,

    /// Create the database if it does not exist
    ///
    /// If the database already exists then it will run the passed in files on the existing
    /// database, but if it does not then the database will be created prior to running the
    /// files. If this is not provided then it will error in the event that the database
    /// is missing.
    #[arg(short, long)]
    pub ensure: bool,
}

impl LoadArgs {
    pub async fn run(&self, _args: &SlArgs) -> anyhow::Result<()> {
        info!("{:?}", self);



        Ok(())
    }
}
