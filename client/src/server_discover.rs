use std::net::SocketAddr;

use common::{messages::JoinReason, networking::send_message};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// Discover a server on the local network.
/// When found, ask the server to join.
/// If the server accepts, return the address of the server.
pub async fn discover_server(
    channel: &mut common::MessageReceiver,
    my_name: &str,
    server_name: Option<&str>,
) -> Option<SocketAddr> {
    info!("Discovering server {:?}", server_name);
    // Initially, we're not expecting a join ack message
    let mut expecting_join_ok_from = None;

    loop {
        let (their_addr, their_name, message) = channel.recv().await?;
        let their_addr = their_addr.ip();
        if server_name.is_none() || their_name == server_name.unwrap() {
            debug!(
                "Received packet from {} ({}): {:?}",
                their_addr, their_name, message
            );
            match message {
                common::messages::Message::Announce { port } => {
                    debug!("It is a server announcement");
                    // If we're not waiting on a join ack, send a join request
                    if expecting_join_ok_from.is_none() {
                        let message = common::messages::Message::JoinQuery {};
                        let their_addr = SocketAddr::new(their_addr, port);

                        send_message(their_addr, my_name, &message).await.ok()?;
                        debug!("Sent join request to {}", their_addr);
                        expecting_join_ok_from = Some(their_addr);
                    }
                }
                common::messages::Message::JoinResponse(reason) => {
                    if expecting_join_ok_from.is_none() {
                        continue;
                    }
                    let expecting_ip_addr = expecting_join_ok_from.unwrap().ip();
                    if their_addr == expecting_ip_addr {
                        debug!("It is a join response");
                        if reason == JoinReason::Accepted {
                            debug!("Server accepted our join request!");
                            return Some(expecting_join_ok_from.unwrap());
                        } else {
                            error!(
                                "Server rejected our join request with this reason: {:?}",
                                reason
                            );
                            expecting_join_ok_from = None;
                            // TODO: Should we exit here?
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
