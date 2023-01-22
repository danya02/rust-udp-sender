use std::{env, time::Duration, net::SocketAddr};

use tokio::{net::UdpSocket, time::Instant};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let rate = args[1].clone();
    let rate = rate.parse::<u64>().expect("First argument must be target number of packets per second");
    let name = common::make_name();
    println!("Starting server with name {}", name);

    let mut last_accounting_period = Instant::now();
    let mut packets_this_period = 0;
    let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind to socket");
    socket.set_broadcast(true).expect("Failed to set broadcast");

    let mut total_sent_packets = 0;
    let addrs = &[SocketAddr::from(([127,255,255,255], 1337))];
    loop {
        let now = Instant::now();
        if now - last_accounting_period > Duration::from_secs(1) {
            let packets_per_second = packets_this_period;
            packets_this_period = 0;
            last_accounting_period = now;
            println!("Packets per second: {}", packets_per_second);
        }
        total_sent_packets += 1;
        let message = common::messages::Message::Ping{ nonce: total_sent_packets, recvs: total_sent_packets };
        common::networking::broadcast_message(&socket, addrs, &name, &message).await.unwrap();

        packets_this_period += 1;
        if packets_this_period > rate {
            println!("manually ratelimiting...");
            tokio::time::sleep(now - last_accounting_period + Duration::from_secs(1)).await;
        }
    }
}


