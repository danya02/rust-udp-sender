pub mod hashlist;
mod args;
mod commands;
mod walk;

use args::Args;
use clap::Parser;


#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{args:?}");

    match args.command {
        args::Subcommand::Hash(options) => {
            commands::make_hash(options).await;
        },
        args::Subcommand::Verify(options) => {
            commands::verify_hash(options).await;
        },
    }
}
