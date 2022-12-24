mod args;
mod transmission;
use args::Args;
use clap::Parser;

fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}
