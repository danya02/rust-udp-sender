mod args;
mod broadcast_presence;
mod broadcaster;
mod files;
mod rate_limiter;

use std::{net::SocketAddr, path::PathBuf};

use args::Args;
use clap::Parser;

use hasher::walk;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::files::run_transmissions;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{args:?}");

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
        let socket_addr = format!("{addr}:{send_port}");
        broadcast_addrs.push(socket_addr.parse().unwrap());
    }

    let listen_addrs: Vec<SocketAddr> =
        vec![SocketAddr::new("0.0.0.0".parse().unwrap(), listen_port)];

    // Create a listener
    let listener = common::networking::make_listener(listen_addrs.clone(), &my_name);

    let rate_limiter = rate_limiter::RateLimiter::new(100, 100000);
    let recv_stat_collector = rate_limiter.get_collector();

    // Create a broadcaster
    let (vip_broadcaster, broadcaster) =
        crate::broadcaster::make_broadcaster(broadcast_addrs.clone(), &my_name, rate_limiter);

    // Create a thread to broadcast our presence
    broadcast_presence::broadcast_presence(vip_broadcaster.clone(), listen_port);

    // Make a listener of JoinQuery messages
    let (mut join_query_listener, listener) = common::channels::filter_branch_pred(
        listener,
        |(_, _, message)| matches!(message, common::messages::Message::JoinQuery {}),
        false,
    );

    // Loop and accept every connection

    let my_name_out = my_name.clone();
    tokio::spawn(async move {
        loop {
            let (src, name, message) = join_query_listener.recv().await.unwrap();
            if let common::messages::Message::JoinQuery {} = message {
                debug!("Received join query from {} ({})", src, name);
                let message =
                    common::messages::Message::JoinResponse(common::messages::JoinReason::Accepted);
                let dest = SocketAddr::new(src.ip(), send_port);
                common::networking::send_message(dest, &my_name_out, &message)
                    .await
                    .ok();
                debug!("Sent join response to {}", dest);
            }
        }
    });

    // Respond to pings with pongs
    let (ping_listener, listener) = common::channels::filter_branch_pred(
        listener,
        |(_, _, message)| matches!(message, common::messages::Message::Ping { .. }),
        false,
    );

    let broadcaster_out = vip_broadcaster.clone();
    tokio::spawn(async move {
        common::ping_reply::reply_to_pings_broadcast(
            common::channels::print(ping_listener),
            broadcaster_out,
            recv_stat_collector,
        )
        .await;
    });
    // Important note: the ping reply thread must not use the broadcaster.
    // The broadcaster uses rate-limiting, and if there is too much traffic
    // the broadcaster will block the ping reply thread, which will cause clients
    // to disconnect.

    // Construct a list of file listing fragments
    let dir: PathBuf = base.clone();
    let file_listing_fragments;
    if let Some(hashlist) = args.hashlist {
        info!("Loading hashlist from {}", hashlist);
        let hashlist: PathBuf = hashlist.parse().unwrap();
        let hashlist = rmp_serde::from_read(std::fs::File::open(hashlist).unwrap()).unwrap();
        file_listing_fragments = files::hashlist_into_file_listing(hashlist);
        debug!(
            "File listing collected, has {} fragments",
            file_listing_fragments.len()
        );
    } else {
        warn!("Building in-memory hashlist, this may take a while");
        let (sender, handle) = walk::collect_entries();
        let dir2 = dir.clone();
        walk::walk_directory_and_hash(dir, dir2, sender).await;
        let hashlist = handle.await.expect("Failed to get hashlist from thread");
        file_listing_fragments = files::hashlist_into_file_listing(hashlist);
        debug!(
            "File listing collected, has {} fragments",
            file_listing_fragments.len()
        );
    }

    tokio::spawn(run_transmissions(
        listener,
        file_listing_fragments,
        broadcaster.clone(),
        vip_broadcaster.clone(),
        base,
    ));

    // Loop over packets
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        //        let (src, name, message) = listener.recv().await.unwrap();
        //        println!("Received unknown packet from {} ({}): {:?}", src, name, message);
    }
}
