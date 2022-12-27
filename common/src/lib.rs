use std::net::SocketAddr;

pub mod magic;
pub mod messages;
pub mod networking;
pub mod channels;
pub mod ping_reply;
pub mod filesystem;

use crate::messages::Message;

type DecodeError = rmp_serde::decode::Error;
pub type MessageGroup = (SocketAddr, String, Message);
pub type MessageReceiver = tokio::sync::mpsc::Receiver<MessageGroup>;
pub type HashType = [u8; 32]; // Sha256

/// Make a random name for an object.
/// 
/// Uses the `petname` crate for human-readable names.
pub fn make_name() -> String {
    petname::petname(3, "-")
}

