use std::net::SocketAddr;

pub mod channels;
pub mod filesystem;
pub mod magic;
pub mod messages;
pub mod networking;
pub mod ping_reply;

use crate::messages::Message;

type DecodeError = rmp_serde::decode::Error;
pub type MessageGroup = (SocketAddr, String, Message);
pub type MessageReceiver = tokio::sync::mpsc::Receiver<MessageGroup>;
pub type HashType = [u8; 32]; // sha256

/// Make a random name for an object.
///
/// Uses the `petname` crate for human-readable names.
pub fn make_name() -> String {
    petname::petname(3, "-")
}
