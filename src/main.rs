use std::{
    env, io,
    net::{SocketAddr, ToSocketAddrs},
    os::unix::prelude::AsRawFd,
    sync::Arc,
};

use tokio::{io::Interest, net::UdpSocket};

#[tokio::main]
async fn main() {
    let _ = env_logger::try_init();
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        print_usage_and_exit(&args[0]);
    }
    let listen_addr = args[1].clone();
    let num_socket = args[2]
        .parse::<usize>()
        .map_err(|e| {
            eprintln!("{}", e);
            print_usage_and_exit(&args[0]);
        })
        .unwrap();

    let listen_addrs: Vec<SocketAddr> = listen_addr
        .to_socket_addrs()
        .map_err(|e| {
            eprintln!("{}", e);
            print_usage_and_exit(&args[0]);
        })
        .unwrap()
        .collect();
    let listen_addr = listen_addrs[0];

    let domain = match listen_addr {
        SocketAddr::V4(_) => socket2::Domain::IPV4,
        SocketAddr::V6(_) => socket2::Domain::IPV6,
    };

    let mut threads = vec![];
    for id in 0..num_socket {
        let socket = socket2::Socket::new(domain, socket2::Type::DGRAM, None).unwrap();
        {
            socket.set_reuse_address(true).unwrap();
            socket.set_reuse_port(true).unwrap();
            socket.set_nonblocking(true).unwrap();
            socket.bind(&listen_addr.into()).unwrap();
        }
        let socket = UdpSocket::from_std(socket.into()).unwrap();
        let handle = tokio::spawn(async move {
            socket_ready_thread(Arc::new(socket), id).await;
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.await.unwrap();
    }
    log::info!("done");
}

async fn socket_ready_thread(socket: Arc<UdpSocket>, reuse_port_id: usize) {
    log::info!(
        "socket_ready_thread started: {{ REUSEPORT ID: {}, socket FD: {} }}",
        reuse_port_id,
        socket.as_raw_fd()
    );
    loop {
        log::trace!(
            "socket_ready_thread waiting for socket ready: {{ reuse_port_id: {} }}",
            reuse_port_id
        );
        let ready = socket.ready(Interest::READABLE).await.unwrap();
        let socket_clone = Arc::clone(&socket);
        if ready.is_readable() {
            tokio::spawn(async move {
                socket_receive_thread(socket_clone, reuse_port_id).await;
            });
        }
    }
}

async fn socket_receive_thread(socket: Arc<UdpSocket>, reuse_port_id: usize) {
    let mut buf = [0u8; 1500];
    log::trace!(
        "socket_receive_thread started: {{ REUSEPORT ID: {} }}",
        reuse_port_id,
    );
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((amt, src)) => {
                log::trace!(
                    "REUSEPORT {} received {} bytes from {}: {{ content_bytes: {:X?}, content_utf8: \"{}\" }}",
                    reuse_port_id,
                    amt,
                    src,
                    &buf[..amt],
                    String::from_utf8_lossy(&buf[0..amt])
                );
                socket.send_to(&buf[0..amt], src).await.unwrap();
            }
            Err(err) => match err.kind() {
                io::ErrorKind::WouldBlock => {
                    break;
                }
                _ => {
                    panic!("{:?}", err);
                }
            },
        };
    }
}

fn print_usage_and_exit(program_name: &str) -> ! {
    eprintln!(
        "Usage: {} <listen address> <number of sockets>",
        program_name
    );
    std::process::exit(1)
}
