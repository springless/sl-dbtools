use clap::Parser;
use sl_dbtools::cli::Args;

fn main() {
    Args::parse().run();
}
