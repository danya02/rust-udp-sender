/// Module for network messages

use serde::{Serialize, Deserialize};

use crate::{DecodeError, HashType};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum JoinReason {
    /// Server accepted the client
    Accepted,
    /// Wrong client name
    WrongName,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum DisconnectReason {
    /// All downloads are complete
    Done,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    /// A request by the client to join a server.
    /// 
    /// The IP address is implied by the UDP packet.
    /// The port is implied: it is the port that the server used to reach the client.
    JoinQuery {
    },

    /// A response to a `JoinQuery`.
    JoinResponse(JoinReason),



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

    /// A record of a single file that the server has.
    /// The server sends several of these to inform the client of the files to transfer.
    FileListing(FileListingFragment),

    /// A request by the client to repeat sending a `FileListingFragment`.
    /// The value is the index of the fragment to repeat.
    /// If the index is out of range, the server can send any of the fragments.
    FileListingRequest{
        idx: u32,
    },

    /// A request to retrieve a chunk of a file.
    /// The server responds with a `FileChunk` message (probably a broadcasting one).
    FileChunkRequest {
        /// The file index.
        /// If this is out of range, the server should send any `FileListing`.
        idx: u32,
        /// The chunk index.
        /// The first chunk is index 0.
        /// If this is out of range, the server should send a `FileListing` for this file.
        chunk: u64,
    },

    /// A chunk of a file.
    /// The server sends this to the client.
    FileChunk(FileChunkData),

    /// A disconnect message.
    /// The client sends this to inform the server that it is no longer listening.
    Disconnect(DisconnectReason),

}

impl Message {
    /// Serialize a message to a byte array.
    pub fn serialize(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).unwrap()
    }

    /// Deserialize a message from a byte array.
    pub fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        rmp_serde::from_slice(data)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileListingFragment {
    /// The zero-based index of this fragment in the file listing.
    pub idx: u32,
    /// The total number of fragments in the file listing.
    pub total: u32,
    /// The path of the file.
    pub path: String,
    /// The size of the file in bytes.
    pub size: u64, // 16 exabytes 
    /// The SHA-256 hash of the file.
    pub hash: HashType,
    /// The size of chunks that the file is split into.
    pub chunk_size: u16, // Up to 64KB (jumbo packet size)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileChunkData {
    /// The index of the file that this chunk is part of.
    pub idx: u32,
    /// The index of this chunk.
    pub chunk: u64,
    /// The data of this chunk.
    pub data: Vec<u8>,
}