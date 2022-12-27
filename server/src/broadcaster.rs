/// Module to deal with broadcasting messages to the network.


use std::net::SocketAddr;
use common::messages::Message;
use tokio::{sync::mpsc, select};


pub type MessageSender = mpsc::Sender<Message>;

/// Make a channel that will broadcast the messages it receives to all given addresses,
/// providing the specified name,
/// and rate-limiting the messages to the given speed.
pub fn make_broadcaster(addrs: Vec<SocketAddr>, name: &str, how_many: usize, mut per_time: tokio::time::Interval) -> MessageSender {
    let (sender, mut receiver) = mpsc::channel(100);
    let name = name.to_string();
    tokio::spawn(async move {
        let mut messages_sent_this_period = 0;
        loop {
            select! {
                Some(message) = receiver.recv() => {
                    if messages_sent_this_period < how_many {
                        messages_sent_this_period += 1;
                        common::networking::broadcast_message(&addrs, &name, &message).await.unwrap();
                    } else {
                        per_time.tick().await; // Rate limiting
                        messages_sent_this_period = 0;
                    }
                },
                _ = per_time.tick() => {
                    messages_sent_this_period = 0;
                },
            }
        }
    });
    sender
}