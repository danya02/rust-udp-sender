/// Functions to deal with UDP transmissions.
/// 
/// This module contains functions to send and receive UDP packets.
/// 
/// Uses tokio for async I/O.

use std::net::SocketAddr;
use tokio::net::UdpSocket;

/// Send a UDP packet to a given address.
pub async fn send_packet(addr: SocketAddr, data: &[u8]) -> Result<(), std::io::Error> {
    let socket = UdpSocket::bind(addr).await?;
    socket.send_to(data, addr).await?;
    Ok(())
}

/// Make a channel to receive UDP packets on any address.
///
/// Binds to the given list of SocketAddrs.
/// Returns a channel that will receive packets.
fn make_listener<I>(addrs: I) -> tokio::sync::mpsc::Receiver<(Vec<u8>, SocketAddr)>
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
                tx.send((buf[..amt].to_vec(), src)).await.unwrap();
            }
        });
    }
    rx
}