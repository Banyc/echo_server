use std::{
    env, io,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    os::unix::prelude::AsRawFd,
    thread,
};

fn main() {
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
        let handle = thread::spawn(move || {
            socket_thread(socket.into(), id);
        });
        threads.push(handle);
    }

    for handle in threads {
        handle.join().unwrap();
    }
    log::info!("done");
}

fn socket_thread(socket: UdpSocket, id: usize) {
    let mut buf = [0u8; 1024];
    log::info!(
        "thread started: {{ thread ID: {}, socket FD: {} }}",
        id,
        socket.as_raw_fd()
    );
    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                log::trace!(
                    "thread {} received {} bytes from {}: {{ content_bytes: {:X?}, content_utf8: \"{}\" }}",
                    id,
                    amt,
                    src,
                    &buf[..amt],
                    String::from_utf8_lossy(&buf[0..amt])
                );
                socket.send_to(&buf[0..amt], src).unwrap();
            }
            Err(err) => match err.kind() {
                io::ErrorKind::WouldBlock => {
                    continue;
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
    std::process::exit(1);
}
