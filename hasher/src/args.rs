use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Args {
    #[clap(subcommand)]
    /// Which mode to run in
    pub command: Subcommand,
}

#[derive(Parser, Debug)]
pub(crate) enum Subcommand {
    /// Hash a directory
    Hash(HashOptions),
    /// Verify a directory
    Verify(VerifyOptions),
}

#[derive(Parser, Debug)]
pub(crate) struct HashOptions {
    /// Path to the directory to hash
    /// If unset, will hash the current directory.
    #[clap(short, long)]
    pub path: Option<String>,

    /// File to write the hashlist to 
    #[clap(short, long)]
    pub file: String,
}

#[derive(Parser, Debug)]
pub(crate) struct VerifyOptions {
    /// Path to the directory to hash
    /// If unset, will hash the current directory.
    #[clap(short, long)]
    pub path: Option<String>,

    /// File to read the hashlist from
    #[clap(short, long)]
    pub file: String,

    /// If set, then files that are in the directory, but not in the hashlist, are not treated as errors.
    #[clap(long, default_value_t = false)]
    pub ignore_new: bool,

    /// If set, then files that are in the hashlist, but not in the directory, are not treated as errors.
    #[clap(long, default_value_t = false)]
    pub ignore_missing: bool,
}