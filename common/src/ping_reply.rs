use std::net::SocketAddr;

use crate::MessageReceiver;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

pub async fn reply_to_pings(mut ping_listener: MessageReceiver, my_name: String, send_port: u16) {
    loop {
        let (src, name, message) = ping_listener.recv().await.unwrap();
        if let crate::messages::Message::Ping{nonce} = message {
            debug!("Received ping from {} ({}) with nonce {}", src, name, nonce);
            let message = crate::messages::Message::Pong{nonce};
            let dest = SocketAddr::new(src.ip(), send_port);
            crate::networking::send_message(dest, &my_name, &message).await.ok();
            debug!("Sent pong to {}", dest);
        }
    }
}