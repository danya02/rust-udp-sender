use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Args {
    /// Port to receive on
    #[clap(short, long, default_value_t = 1337)]
    pub port: u16,

    /// IP addresses to bind to 
    #[clap(short, long, default_value = "0.0.0.0")]
    pub ip: String,

    /// Name of this client. Will show up on server. If unset, generated randomly.
    #[clap(short, long)]
    pub name: Option<String>,
}