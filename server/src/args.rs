use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Args {
    /// Port to transmit on (all the clients must listen on this)
    #[clap(short, long, default_value_t = 1337)]
    pub send_port: u16,

    /// Port to listen on (if unset, will listen on the same port as transmit)
    #[clap(short, long)]
    pub listen_port: Option<u16>,

    /// IP addresses to transmit to, comma-separated (use broadcast addresses)
    #[clap(short, long, default_value = "255.255.255.255")]
    pub ip: String,

    /// Name of this server. Will show up on clients. If unset, generated randomly.
    #[clap(short, long)]
    pub name: Option<String>,
}