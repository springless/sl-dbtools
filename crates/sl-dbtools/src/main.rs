use clap::Parser;
use sl_dbtools::cli::SlArgs;

fn main() {
    SlArgs::parse().run();
}
