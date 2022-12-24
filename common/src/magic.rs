use crate::{messages::Message, DecodeError};

/// Convenience functions for working with magic over the network.

/// The version of the protocol. This must match on both sides of the connection.
static VERSION: u16 = 1;

/// Make a prefix that identifies a peer on the network.
/// Includes the name of the peer and the version of the protocol.
fn make_peer_magic(name: &str) -> Vec<u8> {
    let mut magic = Vec::new();
    magic.extend("RustUDPs".as_bytes());
    magic.extend(name.as_bytes());
    magic.push(0);
    magic.extend(VERSION.to_be_bytes().iter());
    magic
}

/// Errors that can occur when parsing a magic prefix.
#[derive(Debug)]
pub enum MagicError {
    /// This is not a valid magic prefix.
    /// This probably means that the packet is not intended for us, do not say anything.
    InvalidMagic,
    
    /// The version of the protocol does not match.
    /// The value included is their version.
    /// You might want a warning upon seeing this.
    InvalidVersion(u16),

    /// The magic prefix was valid, but the packet was otherwise invalid.
    DecodeError(DecodeError),

}

/// Extract the contents of a packet, stripping the magic prefix.
/// Returns the name of the peer and the rest of the packet.
pub fn parse_magic(data: &[u8]) -> Result<(String, &[u8]), MagicError> {
    // The first 8 bytes are the magic prefix
    let (magic, data) = data.split_at(8);
    if magic != "RustUDPs".as_bytes() {
        return Err(MagicError::InvalidMagic);
    }
    // The next bytes until the null byte are the name
    // Find the null byte or return an error
    let maybe_zero_position = data.iter().position(|&x| x == 0);
    let zero_position = match maybe_zero_position {
        Some(zero_position) => zero_position,
        None => return Err(MagicError::InvalidMagic),
    };
    let (name, data) = data.split_at(zero_position);
    let name = String::from_utf8_lossy(name).to_string();
    // The next one byte is zero, then 2 bytes are the version
    let (version, data) = data.split_at(3);
    let version = u16::from_be_bytes([version[1], version[2]]);
    if version != VERSION {
        return Err(MagicError::InvalidVersion(version));
    }
    Ok((name, data))
}

/// Make a packet with the magic prefix from the given message
pub fn make_magic_packet(name: &str, data: &Message) -> Vec<u8> {
    let mut packet = make_peer_magic(name);
    let data = data.serialize();
    packet.extend(data);
    packet
}

/// Parse a packet with the magic prefix into a message
/// Returns the name of the peer and the message
pub fn parse_magic_packet(data: &[u8]) -> Result<(String, Message), MagicError> {
    let (name, data) = parse_magic(data)?;
    let message = Message::deserialize(data).map_err(|e| MagicError::DecodeError(e))?;
    Ok((name, message))
}