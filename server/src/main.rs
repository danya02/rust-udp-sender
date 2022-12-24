mod args;
use args::Args;
use clap::Parser;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    let my_name = match args.name {
        Some(name) => name,
        None => common::make_name(),
    };

    let mut addresses = vec![];
    for addr in args.ip.split(",") {
        let socket_addr = format!("{}:{}", addr, args.port);
        addresses.push(socket_addr.parse().unwrap());
    }

    let listen_port = match args.listen_port {
        Some(port) => port,
        None => args.port,
    };




    loop {
        // Broadcast a packet
        let message = common::messages::Message::Announce { port: listen_port };
        common::networking::broadcast_packet(&addresses, &common::magic::make_magic_packet(&my_name, &message)).await.unwrap();
        println!("Sent a packet");
        // Wait a bit
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
