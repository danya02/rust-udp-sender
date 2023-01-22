use std::net::SocketAddr;

use crate::{messages::Message, MessageReceiver};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use tokio::sync::mpsc;

pub async fn reply_to_pings(
    mut ping_listener: MessageReceiver,
    my_name: String,
    send_port: u16,
    recv_stat_collector: mpsc::Sender<(String, u64)>,
) {
    loop {
        let (src, name, message) = ping_listener.recv().await.unwrap();
        if let crate::messages::Message::Ping { nonce, recvs } = message {
            debug!("Received ping from {} ({}) with nonce {}", src, name, nonce);
            recv_stat_collector.send((name, recvs)).await.ok();
            let message = crate::messages::Message::Pong { nonce };
            let dest = SocketAddr::new(src.ip(), send_port);
            crate::networking::send_message(dest, &my_name, &message)
                .await
                .ok();
            debug!("Sent pong to {}", dest);
        }
    }
}

pub async fn reply_to_pings_broadcast(
    mut ping_listener: MessageReceiver,
    broadcaster: mpsc::Sender<Message>,
    recv_stat_collector: mpsc::Sender<(String, u64)>,
) {
    loop {
        let (src, name, message) = ping_listener.recv().await.unwrap();
        if let crate::messages::Message::Ping { nonce, recvs } = message {
            info!("Received ping from {} ({}) with nonce {}", src, name, nonce);
            recv_stat_collector.send((name, recvs)).await.ok();
            let message = crate::messages::Message::Pong { nonce };
            info!("Sent pong {message:?}");
            broadcaster.send(message).await.ok();
        }
    }
}
