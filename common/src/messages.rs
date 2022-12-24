/// Module for network messages

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Message {
    /// Broadcasted by a server to announce its presence.
    /// 
    /// The IP address is implied by the UDP packet.
    /// The port is used by clients to talk back to the server.
    Announce {
        /// The port that the server is listening on for return communications.
        port: u16,
    },

    /// A request to join a server.
    /// 
    /// The IP address is implied by the UDP packet.
    /// The port is used by the server to talk back to the client.
    JoinQuery {
        port: u16,
    },

    /// A response to a `JoinQuery`.
    JoinResponse {
        /// Whether the server has accepted the client.
        accepted: bool,
    },


    /// A ping request. Whoever sends this expects a `Pong` in response.
    Ping {
        /// Random number to identify this ping.
        nonce: u64,
    },

    /// A response to a `Ping`.
    Pong {
        /// The nonce from the `Ping` that this is a response to.
        nonce: u64,
    },
}