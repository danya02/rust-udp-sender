use std::net::SocketAddr;

use common::{messages::Message, MessageReceiver, networking::send_message};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


/// Periodically send out pings to the server, and listen for pongs.
/// If the number of missed pongs exceeds the threshold, the program will exit.
pub(crate) async fn pong_listener(mut pong_listener: MessageReceiver, mut ping_interval: tokio::time::Interval, ping_threshold: u32, addr: SocketAddr, my_name: String) {
    let mut missed_pings = 0;
    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                // If the server spent too long without replying with a ping,
                // it either means that the server is down, or that
                // there are no in-flight pings.
                // So send a ping now.

                missed_pings += 1;
                if missed_pings > ping_threshold {
                    eprintln!("Missed too many pings, exiting");
                    std::process::exit(1);
                }

                let nonce = rand::random();
                let message = Message::Ping{nonce};
                send_message(addr, &my_name, &message).await.ok();
                debug!("Sent ping to {} with nonce {}", addr, nonce);
            }
            Some((src, name, message)) = pong_listener.recv() => {
                // If we receive a pong, reset the missed pings counter.
                if let Message::Pong{nonce} = message {
                    debug!("Received pong from {} ({}) with nonce {}", src, name, nonce);
                    missed_pings = 0;
                }
            }
        }
    }
}