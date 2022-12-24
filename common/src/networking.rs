/// Functions to deal with UDP transmissions.
/// 
/// This module contains functions to send and receive UDP packets.
/// 
/// Uses tokio for async I/O.

use std::net::SocketAddr;
use tokio::net::UdpSocket;

use crate::{messages::Message, magic::{parse_magic_packet, MagicError}};

/// Send a UDP packet to a given address.
pub async fn send_packet(addr: SocketAddr, data: &[u8]) -> Result<(), std::io::Error> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.send_to(data, addr).await?;
    Ok(())
}

/// Make a channel to receive messages on any of these addresses.
///
/// Binds to the given list of SocketAddrs.
/// Returns a channel that will receive packets.
pub fn make_listener<I>(addrs: I) -> tokio::sync::mpsc::Receiver<(SocketAddr, String, Message)>
where
    I: IntoIterator<Item = SocketAddr>,
{
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    for addr in addrs {
        let tx = tx.clone();
        tokio::spawn(async move {
            let socket = UdpSocket::bind(addr).await.unwrap();
            loop {
                let mut buf = [0; 1024];
                let (amt, src) = socket.recv_from(&mut buf).await.unwrap();
                
                let data = &buf[..amt];
                let maybe_magic_decoded = parse_magic_packet(&data);
                match maybe_magic_decoded {
                    Ok((name, message)) => {
                        tx.send((src, name, message)).await.unwrap();
                    },
//                    Err(MagicError::InvalidMagic) => {}, // Ignore
//                    Err(MagicError::InvalidVersion(v)) => {}, // Ignore
//                    Err(MagicError::DecodeError(e)) => {
//                        eprintln!("Error decoding packet: {}", e);
//                    }
                    Err(e) => {
                        eprintln!("Error in: {:?}", e);
                    }
                }
            }
        });
    }
    rx
}

/// Broadcast a packet to a list of addresses.
pub async fn broadcast_packet(addrs: &[SocketAddr], data: &[u8]) -> Result<(), std::io::Error> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    for addr in addrs {
        socket.send_to(data, addr).await?;
    }
    Ok(())
}