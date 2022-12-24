mod args;
mod server_discover;
mod pong_listener;

use std::net::SocketAddr;

use args::Args;
use clap::Parser;


#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{:?}", args);

    let my_name = match args.name {
        Some(name) => name,
        None => common::make_name(),
    };

    eprintln!("Starting client as {}", my_name);

    // Create a listener
    let addresses = vec![ SocketAddr::new(args.ip.parse().unwrap(), args.port) ];
    let mut listener = common::networking::make_listener(addresses, &my_name);

    // Discover the server
    let server_addr = server_discover::discover_server(&mut listener, &my_name, args.server_name.as_deref()).await.unwrap();
    let server_port = server_addr.port();

    // Respond to pings
    let (ping_listener, listener) = common::channels::filter_branch_pred(listener,
        |(_, _, message)| {
            matches!(message, common::messages::Message::Ping{..})
        }, false
    );

    let my_name_out = my_name.clone();
    tokio::spawn(async move {
        common::ping_reply::reply_to_pings(ping_listener, my_name_out, server_port).await;
    });

    // Periodically send a ping, listening for pongs

    let (pong_listener, mut listener) = common::channels::filter_branch_pred(listener,
        |(_, _, message)| {
            matches!(message, common::messages::Message::Pong{..})
        }, false
    );

    let my_name_out = my_name.clone();
    let ping_interval = std::time::Duration::from_secs(1);
    let ping_interval = tokio::time::interval(ping_interval);
    tokio::spawn(async move {
        pong_listener::pong_listener(pong_listener, ping_interval, 5, server_addr, my_name_out).await;
    });

    // Loop over packets
    loop {
        let (src, name, message) = listener.recv().await.unwrap();
        println!("Received packet from {} ({}): {:?}", src, name, message);
    }
}
