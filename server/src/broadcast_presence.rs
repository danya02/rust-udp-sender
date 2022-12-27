use common::networking::broadcast_message;
use tokio::time::Duration;

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use crate::broadcaster;


/// Periodically broadcast presence to the broadcast addresses
pub fn broadcast_presence(broadcaster: broadcaster::MessageSender, my_port: u16) -> tokio::task::JoinHandle<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let message = common::messages::Message::Announce { port: my_port };
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            broadcaster.send(message.clone()).await;
        }
    })
}