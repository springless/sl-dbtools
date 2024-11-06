use clap::Parser;
use sl_dbtools::cli::SlArgs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    SlArgs::parse().run().await
}
