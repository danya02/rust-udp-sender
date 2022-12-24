mod args;
use args::Args;
use clap::Parser;

fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    let my_name = match args.name {
        Some(name) => name,
        None => common::make_name(),
    };

    eprintln!("Starting client as {}", my_name);

    // Create a socket
    let socket = match std::net::UdpSocket::bind(format!("{}:{}", args.ip, args.port)) {
        Ok(socket) => socket,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Receive a packet
    let mut buf = [0; 1024];
    let (amt, src) = match socket.recv_from(&mut buf) {
        Ok((amt, src)) => (amt, src),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    // Print the packet
    println!("Received {} bytes from {}", amt, src);

}
