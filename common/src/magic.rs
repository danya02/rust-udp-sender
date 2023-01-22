use crc::Crc;

use crate::{messages::Message, DecodeError};

/// Convenience functions for working with magic over the network.

/// The version of the protocol. This must match on both sides of the connection.
static VERSION: u16 = 1;

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

    /// There is a length mismatch.
    /// This probably means that there is network corruption or truncation. Consider using a smaller packet size.
    /// The first value is the expected length, the second is the actual length.
    LengthMismatch(u16, u16),

    /// There is a hash mismatch.
    /// This probably means that there is network corruption.
    HashMismatch,

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
    // The next 2 bytes are the length
    let (length, data) = data.split_at(2);
    let length = u16::from_be_bytes([length[0], length[1]]);
    // The next bytes are the hash
    let hash_length = std::mem::size_of::<crate::HashType>();
    let (hash, data) = data.split_at(hash_length);
    // The rest is the data
    if data.len() as u16 != length {
        return Err(MagicError::LengthMismatch(length, data.len() as u16));
    }

    // Hash the data
    let crc = Crc::<u32>::new(&crc::CRC_32_CKSUM);
    let mut hasher = crc.digest();
    hasher.update(data);
    let hash2 = hasher.finalize();
    let hash2 = hash2.to_be_bytes();
    if hash != hash2.as_slice() {
        return Err(MagicError::HashMismatch);
    }

    Ok((name, data))
}

/// Make a packet with the magic prefix from the given message
pub fn make_magic_packet(name: &str, data: &Message) -> Vec<u8> {
    let data = data.serialize();

    let mut packet = Vec::new();
    // Magic prefix
    packet.extend("RustUDPs".as_bytes());
    packet.extend(name.as_bytes());
    packet.push(0);
    packet.extend(VERSION.to_be_bytes().iter());
    // Length
    packet.extend((data.len() as u16).to_be_bytes().iter());
    // Hash
    let crc = Crc::<u32>::new(&crc::CRC_32_CKSUM);
    let mut hasher = crc.digest();
    hasher.update(&data);
    let hash = hasher.finalize();
    packet.extend(hash.to_be_bytes());
    // Data
    packet.extend(data);
    packet
}

/// Parse a packet with the magic prefix into a message
/// Returns the name of the peer and the message
pub fn parse_magic_packet(data: &[u8]) -> Result<(String, Message), MagicError> {
    let (name, data) = parse_magic(data)?;
    let message = Message::deserialize(data).map_err(MagicError::DecodeError)?;
    Ok((name, message))
}
