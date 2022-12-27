use std::net::SocketAddr;

use common::{messages::Message, networking::send_message};

/// Structure to hold info on how to send info to the server

#[derive(Debug, Clone)]
pub struct ServerCommunicator {
    /// The server's SocketAddr
    addr: SocketAddr,
    /// My name
    my_name: String,
}

impl ServerCommunicator {
    /// Create a new ServerCommunicator
    pub fn new(addr: SocketAddr, my_name: String) -> Self {
        Self { addr, my_name }
    }

    /// Send a message to the server
    pub async fn send_message(&self, message: &Message) {
        send_message(self.addr, &self.my_name, message).await.expect("Error while sending message to server over UDP");
    }
}