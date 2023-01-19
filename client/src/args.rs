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

    /// Name of the server to connect to. If unset, will connect to the first server that responds.
    #[clap(short, long)]
    pub server_name: Option<String>,

    /// Request a new file chunk that we're missing every N microseconds.
    /// This value is per file: if you have 10 files, and this value is 1000000, you will request 10 chunks per second.
    /// If set to 0, will not request any chunks, and will only rely on broadcasted chunks.
    #[clap(short, long, default_value_t = 10000)]
    pub request_interval_us: u64,
}