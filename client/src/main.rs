mod args;
use std::net::SocketAddr;

use args::Args;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    let my_name = match args.name {
        Some(name) => name,
        None => common::make_name(),
    };

    eprintln!("Starting client as {}", my_name);

    // Create a listener
    let addresses = vec![ SocketAddr::new(args.ip.parse().unwrap(), args.port) ];
    let mut listener = common::networking::make_listener(addresses);

    // Loop over packets
    loop {
        let (src, name, message) = listener.recv().await.unwrap();
        println!("Received packet from {} ({}): {:?}", src, name, message);
    }
}
