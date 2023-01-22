use common::{messages::Message, MessageReceiver};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::comms::ServerCommunicator;

/// Periodically send out pings to the server, and listen for pongs.
/// If the number of missed pongs exceeds the threshold, the program will exit.
pub(crate) async fn pong_listener(
    mut pong_listener: MessageReceiver,
    mut ping_interval: tokio::time::Interval,
    ping_threshold: u32,
    comm: ServerCommunicator,
    recv_packets_counter: tokio::sync::watch::Receiver<u64>,
    recv_packets_count_reset: tokio::sync::watch::Sender<()>,
) {
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
                let recv_packets = *recv_packets_counter.borrow();
                let message = Message::Ping{nonce, recvs: recv_packets};
                comm.send_message(&message).await;
                debug!("Sent ping with nonce {nonce} and recvs {recv_packets}");

                // When we send a ping, reset the packet counter.
                recv_packets_count_reset.send(()).unwrap();
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
