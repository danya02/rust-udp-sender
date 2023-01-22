#![feature(is_some_and)]

mod args;
mod channels;
mod comms;
mod download;
mod packet_counter;
mod pong_listener;
mod progress_indicator;
mod server_discover;
mod server_state;
mod server_state_initialization;

use std::{net::SocketAddr, path::PathBuf};

use args::Args;
use clap::Parser;

use common::messages::{DisconnectReason, Message};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use tokio::sync::mpsc;

use crate::progress_indicator::ProgressIndicator;

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    println!("{args:?}");

    let my_name = match args.name {
        Some(name) => name,
        None => common::make_name(),
    };

    eprintln!("Starting client as {my_name}");

    // Create a listener
    let addresses = vec![SocketAddr::new(args.ip.parse().unwrap(), args.port)];
    let og_listener = common::networking::make_listener(addresses, &my_name);

    // Count the packets
    let (sender, mut listener) = mpsc::channel(100);
    let (count_sender, count_receiver) = tokio::sync::watch::channel(0);
    let (reset_sender, reset_receiver) = tokio::sync::watch::channel(());
    tokio::spawn(packet_counter::count_packets(
        og_listener,
        sender,
        count_sender,
        reset_receiver,
    ));

    // Discover the server
    let server_addr =
        server_discover::discover_server(&mut listener, &my_name, args.server_name.as_deref())
            .await
            .unwrap();
    let server_port = server_addr.port();

    let server_comm = comms::ServerCommunicator::new(server_addr, my_name.clone());

    // Respond to pings
    let (ping_listener, listener) = common::channels::filter_branch_pred(
        listener,
        |(_, _, message)| matches!(message, common::messages::Message::Ping { .. }),
        false,
    );

    // TODO: at present, we don't care about the server's pings.
    // Because of this, we ignore the packet stats that we get from the server

    let (sender, receiver) = mpsc::channel(100);
    common::channels::drain(receiver);

    let my_name_out = my_name.clone();
    tokio::spawn(async move {
        common::ping_reply::reply_to_pings(ping_listener, my_name_out, server_port, sender).await;
    });

    // Periodically send a ping, listening for pongs

    let (pong_listener, mut listener) = common::channels::filter_branch_pred(
        listener,
        |(_, _, message)| matches!(message, common::messages::Message::Pong { .. }),
        false,
    );

    let _my_name_out = my_name.clone();
    let ping_interval = std::time::Duration::from_secs(1);
    let ping_interval = tokio::time::interval(ping_interval);
    let comm = server_comm.clone();

    tokio::spawn(async move {
        pong_listener::pong_listener(
            pong_listener,
            ping_interval,
            10,
            comm,
            count_receiver,
            reset_sender,
        )
        .await;
    });

    // We now need to get the initial server state.
    let state =
        server_state_initialization::initialize_state(&mut listener, server_comm.clone()).await;

    // Initialize the progress indicator
    let mut indicator = ProgressIndicator::new(&state);

    // When we know what we need to download: for each file, start a download thread
    let (download_listeners, listener) = crate::channels::split_by_files(listener, state.clone());
    let mut join_handles = vec![];
    let request_interval_us = args.request_interval_us;
    for (file, listener) in state.files.iter().zip(download_listeners) {
        let comm = server_comm.clone();
        let (file, chunks) = file.clone();
        let progress_sender = indicator.event_tx();
        let handle = tokio::spawn(async move {
            // Allocate the file
            common::filesystem::allocate(&PathBuf::from(&file.path), file.size)
                .await
                .expect("Failed to allocate file");
            download::download_file(
                listener,
                comm,
                file,
                chunks,
                progress_sender,
                request_interval_us,
            )
            .await;
        });
        join_handles.push(handle);
    }

    //common::channels::drain_with_print(listener);
    common::channels::drain(listener); // Collects Announce and FileListing

    indicator
        .run(true)
        .await
        .expect("Failed to download all files");

    // Wait for all downloads to finish
    for handle in join_handles {
        handle.await.unwrap();
    }

    println!("All downloads finished!");
    server_comm
        .send_message(&Message::Disconnect(DisconnectReason::Done))
        .await;
}
