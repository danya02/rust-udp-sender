use std::net::SocketAddr;
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use common::messages::Message;

#[tokio::main]
async fn main() {
    let mut peer_packet_counts = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut last_heard_from: HashMap<String, Instant> = HashMap::new();

    let mut listener = common::networking::make_listener(vec![SocketAddr::from(([0, 0, 0, 0], 1337))], "...");
    loop {
        tokio::select!{
            _ = interval.tick() => {
                println!("Stats: ");
                for (name, (they_sent, we_recv)) in peer_packet_counts.iter() {
                    println!("{}: {} sent, {} recv, success fraction {}", name, they_sent, we_recv, *we_recv as f64 / *they_sent as f64);
                }

                let now = tokio::time::Instant::now();
                let to_delete = last_heard_from.iter().filter(|(_, last_heard)| now - **last_heard > Duration::from_secs(10)).map(|(name, _)| name.clone()).collect::<Vec<_>>();
                for name in to_delete {
                    println!("{} timed out", name);
                    peer_packet_counts.remove(&name);
                    last_heard_from.remove(&name);
                }
                println!("");
            }
            Some((_src, name, message)) = listener.recv() => {
                if let Message::Ping{ nonce, recvs } = message {
                    if nonce == recvs { // This should never happen naturally: these packets are only sent by the bandwidth test server
                        last_heard_from.insert(name.clone(), tokio::time::Instant::now());
                        if peer_packet_counts.contains_key(&name) {
                            let (they_sent, we_recv) = peer_packet_counts.get_mut(&name).unwrap();
                            *they_sent = nonce;
                            *we_recv += 1;
                        } else {
                            println!("New peer: {}", name);
                            peer_packet_counts.insert(name, (nonce, 1));
                        }
                    }
                }
            }
        }
    }
}