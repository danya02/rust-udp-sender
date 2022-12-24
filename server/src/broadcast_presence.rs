use std::net::SocketAddr;
use common::networking::broadcast_message;
use tokio::time::Duration;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};


/// Periodically broadcast presence to the broadcast addresses
pub fn broadcast_presence(addrs: Vec<SocketAddr>, my_name: &str, my_port: u16) -> tokio::task::JoinHandle<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let my_name = my_name.to_string();
    let message = common::messages::Message::Announce { port: my_port };
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            broadcast_message(&addrs, &my_name, &message).await.expect("Error while broadcasting presence");
        }
    })
}