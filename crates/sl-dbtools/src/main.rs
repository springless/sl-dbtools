use clap::Parser;
use sl_dbtools::cli::SlArgs;

fn main() -> anyhow::Result<()> {
    SlArgs::parse().run()
}
