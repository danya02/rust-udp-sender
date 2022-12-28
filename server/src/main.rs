mod args;
mod broadcast_presence;
mod files;
mod broadcaster;

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use args::Args;
use clap::Parser;

use hasher::walk;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use tokio::time::interval;

use crate::files::run_transmissions;


#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{:?}", args);

    let my_name = match args.name {
        Some(name) => name,
        None => common::make_name(),
    };
    
    info!("Starting server as {}", my_name);

    let send_port = args.send_port;

    let base = PathBuf::from(args.dir);

    let listen_port = match args.listen_port {
        Some(port) => port,
        None => send_port,
    };

    let mut broadcast_addrs: Vec<SocketAddr> = vec![];
    for addr in args.ip.split(',') {
        let socket_addr = format!("{}:{}", addr, send_port);
        broadcast_addrs.push(socket_addr.parse().unwrap());

    }

    let listen_addrs: Vec<SocketAddr> = vec![SocketAddr::new("0.0.0.0".parse().unwrap(), listen_port)];

    // Create a listener
    let listener = common::networking::make_listener(listen_addrs.clone(), &my_name);

    // Create a broadcaster
    let broadcaster = crate::broadcaster::make_broadcaster(broadcast_addrs.clone(), &my_name, 10000, interval(Duration::from_secs(1)));

    // Create a thread to broadcast our presence
    broadcast_presence::broadcast_presence(broadcaster.clone(), listen_port);

    // Make a listener of JoinQuery messages
    let (mut join_query_listener, listener) = common::channels::filter_branch_pred(listener,
        |(_, _, message)| {
            matches!(message, common::messages::Message::JoinQuery{})
        }, false
    );

    // Loop and accept every connection

    let my_name_out = my_name.clone();
    tokio::spawn(async move {
        loop {
            let (src, name, message) = join_query_listener.recv().await.unwrap();
            if let common::messages::Message::JoinQuery{} = message {
                debug!("Received join query from {} ({})", src, name);
                let message = common::messages::Message::JoinResponse(common::messages::JoinReason::Accepted);
                let dest = SocketAddr::new(src.ip(), send_port);
                common::networking::send_message(dest, &my_name_out, &message).await.ok();
                debug!("Sent join response to {}", dest);
            }
        }
    });

    // Respond to pings with pongs
    let (ping_listener, mut listener) = common::channels::filter_branch_pred(listener,
        |(_, _, message)| {
            matches!(message, common::messages::Message::Ping{..})
        }, false
    );

    let my_name_out = my_name.clone();
    tokio::spawn(async move {
        common::ping_reply::reply_to_pings(ping_listener, my_name_out, send_port).await;
    });

 
    // Construct a list of file listing fragments
    let dir: PathBuf = base.clone();
    let file_listing_fragments;
    if let Some(hashlist) = args.hashlist {
        info!("Loading hashlist from {}", hashlist);
        let hashlist: PathBuf = hashlist.parse().unwrap();
        let hashlist = rmp_serde::from_read(std::fs::File::open(hashlist).unwrap()).unwrap();
        file_listing_fragments = files::hashlist_into_file_listing(hashlist);
        debug!("File listing collected, has {} fragments", file_listing_fragments.len());
    }
    else {
        warn!("Building in-memory hashlist, this may take a while");
        let (sender, handle) = walk::collect_entries();
        let dir2 = dir.clone();
        walk::walk_directory_and_hash(dir, dir2, sender).await;
        let hashlist = handle.await.expect("Failed to get hashlist from thread");
        file_listing_fragments = files::hashlist_into_file_listing(hashlist);
        debug!("File listing collected, has {} fragments", file_listing_fragments.len());
    }

    tokio::spawn(run_transmissions(listener, file_listing_fragments, broadcaster.clone(), base));


    // Loop over packets
    loop {
//        let (src, name, message) = listener.recv().await.unwrap();
//        println!("Received unknown packet from {} ({}): {:?}", src, name, message);
    }
}
