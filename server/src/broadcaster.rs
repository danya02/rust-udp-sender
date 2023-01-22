use common::messages::Message;
/// Module to deal with broadcasting messages to the network.
use std::net::SocketAddr;
use tokio::{select, sync::mpsc, net::UdpSocket};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::rate_limiter::RateLimiter;

pub type MessageSender = mpsc::Sender<Message>;

/// Make a channel that will broadcast the messages it receives to all given addresses,
/// providing the specified name,
/// and rate-limiting the messages to the given speed.
///
/// Produces two MessageSenders. The first one is for important messages that should be sent
/// immediately, and the second one is for normal messages that should be rate-limited.
pub fn make_broadcaster(
    addrs: Vec<SocketAddr>,
    name: &str,
    mut rate_limiter: RateLimiter,
) -> (MessageSender, MessageSender) {
    let (sender, mut receiver) = mpsc::channel(100);
    let (vip_sender, mut vip_receiver) = mpsc::channel(100);
    let name = name.to_string();
    tokio::spawn(async move {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        socket.set_broadcast(true).unwrap();
        loop {
            select! {
                Some(message) = vip_receiver.recv() => {
                    // Important messages are sent immediately, without advancing the rate limiter
                    common::networking::broadcast_message(&socket, &addrs, &name, &message).await.unwrap();
                    log::debug!("Message {message:?} on wire as VIP");
                },
                Some(message) = receiver.recv() => {
                        rate_limiter.on_packet().await;
                        common::networking::broadcast_message(&socket, &addrs, &name, &message).await.unwrap();
                        log::debug!("Message {message:?} on wire");
                },
            }
        }
    });
    (vip_sender, sender)
}
