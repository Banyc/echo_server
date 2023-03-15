use std::{
    io,
    net::{SocketAddr, UdpSocket},
    os::fd::AsRawFd,
    thread,
};

use tracing::{error, info, instrument, trace};

use crate::domain;

#[instrument]
pub fn spawn_threads(
    listen_addr: SocketAddr,
    num_sockets: usize,
) -> io::Result<Vec<thread::JoinHandle<io::Result<()>>>> {
    let domain = domain(&listen_addr);

    let mut threads = vec![];
    let listen_addr = listen_addr.into();
    for id in 0..num_sockets {
        let socket = socket2::Socket::new(domain, socket2::Type::DGRAM, None)?;
        {
            socket.set_reuse_address(true)?;
            socket.set_reuse_port(true)?;
            socket.set_nonblocking(false)?;
            socket.bind(&listen_addr)?;
        }
        let handle = thread::spawn(move || echo(socket.into(), id));
        threads.push(handle);
    }

    Ok(threads)
}

#[instrument(skip(socket))]
pub fn echo(socket: UdpSocket, id: usize) -> io::Result<()> {
    let mut buf = [0u8; 1024 * 64];
    let addr = socket.local_addr()?;
    info!(?id, fd = ?socket.as_raw_fd(), ?addr, "UDP thread started");
    loop {
        match socket.recv_from(&mut buf) {
            Ok((amt, src)) => {
                let pkt = &buf[0..amt];
                trace!(?amt, ?src, ?pkt, "UDP thread received data");
                socket.send_to(pkt, src)?;
            }
            Err(err) => match err.kind() {
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted => {
                    continue;
                }
                _ => {
                    error!(?err, "UDP thread failed");
                    return Err(err);
                }
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{ToSocketAddrs, UdpSocket};

    const LISTENER_PORT: u16 = 7;

    #[test]
    fn test() {
        let _ = tracing_subscriber::fmt::try_init();
        let addr = format!("localhost:{}", LISTENER_PORT)
            .to_socket_addrs()
            .unwrap()
            .collect::<Vec<_>>()[0];

        let _ = spawn_threads(addr, 1).unwrap();

        let socket = UdpSocket::bind("localhost:0").unwrap();
        let pkt = b"hello world";
        socket.send_to(pkt, addr).unwrap();
        let mut buf = [0u8; 1024];
        let (amt, _) = socket.recv_from(&mut buf).unwrap();
        assert_eq!(amt, pkt.len());
        assert_eq!(&buf[0..amt], pkt);
    }
}
