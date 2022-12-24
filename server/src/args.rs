use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct Args {
    /// Port to transmit on
    #[clap(short, long, default_value_t = 1337)]
    port: u16,

    /// IP addresses to transmit to, comma-separated (use broadcast addresses)
    #[clap(short, long, default_value = "255.255.255.255")]
    ip: String,
}