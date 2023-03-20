use std::{
    env,
    net::{SocketAddr, ToSocketAddrs},
};

use echo_server::{tcp_echo, udp_echo};
use thiserror::Error;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Parse command line arguments.
    let args = match parse() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("error: {}", err);
            let program_name = env::args().next().unwrap();
            print_usage_and_exit(&program_name);
        }
    };

    // Spawn echo servers.
    let tokio_threads = tcp_echo::spawn_threads(args.listen_addr, args.num_tcp_sockets).unwrap();
    let std_threads = udp_echo::spawn_threads(args.listen_addr, args.num_udp_sockets).unwrap();

    // Wait for threads to finish.
    let mut set = tokio::task::JoinSet::new();
    for handle in tokio_threads {
        set.spawn(handle);
    }
    while let Some(res) = set.join_next().await {
        res.unwrap().unwrap().unwrap();
    }
    for handle in std_threads {
        handle.join().unwrap().unwrap();
    }
}

struct Args {
    listen_addr: SocketAddr,
    num_tcp_sockets: usize,
    num_udp_sockets: usize,
}

#[derive(Debug, Error)]
enum ParseError {
    #[error("invalid listen address")]
    ListenAddress(std::io::Error),
    #[error("invalid number of TCP sockets")]
    NumberOfTcpSockets(std::num::ParseIntError),
    #[error("invalid number of UDP sockets")]
    NumberOfUdpSockets(std::num::ParseIntError),
}

fn parse() -> Result<Args, ParseError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        print_usage_and_exit(&args[0]);
    }
    let listen_addr = args[1].clone();
    let num_tcp_sockets = args[2]
        .parse::<usize>()
        .map_err(ParseError::NumberOfTcpSockets)?;
    let num_udp_sockets = args[3]
        .parse::<usize>()
        .map_err(ParseError::NumberOfUdpSockets)?;

    let listen_addrs: Vec<SocketAddr> = listen_addr
        .to_socket_addrs()
        .map_err(ParseError::ListenAddress)?
        .collect();
    let listen_addr = listen_addrs[0];

    Ok(Args {
        listen_addr,
        num_tcp_sockets,
        num_udp_sockets,
    })
}

fn print_usage_and_exit(program_name: &str) -> ! {
    eprintln!(
        "Usage: RUST_LOG=<logging level> {} <listen address> <number of TCP sockets> <number of UDP sockets>",
        program_name
    );
    eprintln!("Hint: num_cpus={}", num_cpus::get());
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_payload() {
        let buf = [0u8; 1024];
        let amt = 0;
        let src = "127.0.0.2:12341".parse::<SocketAddr>().unwrap();
        let socket =
            socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, None).unwrap();
        socket.send_to(&buf[0..amt], &src.into()).unwrap();
    }
}
