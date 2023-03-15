use std::net::SocketAddr;
use std::{io, os::fd::AsRawFd};

use tracing::{error, info, instrument, trace};

use crate::domain;

const BACKLOG: usize = 1024;

#[instrument]
pub fn spawn_threads(
    listen_addr: SocketAddr,
    num_sockets: usize,
) -> io::Result<Vec<tokio::task::JoinHandle<io::Result<()>>>> {
    let domain = domain(&listen_addr);

    let mut threads = vec![];
    let listen_addr = listen_addr.into();
    for id in 0..num_sockets {
        let socket = socket2::Socket::new(domain, socket2::Type::STREAM, None)?;
        {
            socket.set_reuse_address(true)?;
            socket.set_reuse_port(true)?;
            socket.set_nonblocking(true)?;
            socket.bind(&listen_addr)?;
            socket.listen(BACKLOG as i32)?;
        }
        let tokio_listener = tokio::net::TcpListener::from_std(socket.into())?;
        let handle = tokio::spawn(echo(tokio_listener, id));
        threads.push(handle);
    }

    Ok(threads)
}

#[instrument(skip(socket))]
pub async fn echo(socket: tokio::net::TcpListener, id: usize) -> io::Result<()> {
    let addr = socket.local_addr()?;
    info!(?id, fd = ?socket.as_raw_fd(), ?addr, "TCP thread started");
    loop {
        trace!("TCP thread waiting for connection");
        let (stream, src) = socket.accept().await?;
        trace!(?src, "TCP thread accepted connection");
        tokio::spawn(async move {
            if let Err(err) = echo_stream(stream, id).await {
                error!(?err, "TCP thread failed");
            }
        });
    }
}

#[instrument(skip(socket))]
pub async fn echo_stream(mut socket: tokio::net::TcpStream, id: usize) -> io::Result<()> {
    let (mut reader, mut writer) = socket.split();
    tokio::io::copy(&mut reader, &mut writer).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpStream, ToSocketAddrs},
    };

    const LISTENER_PORT: u16 = 7;

    #[tokio::test(flavor = "multi_thread")]
    async fn test() {
        let _ = tracing_subscriber::fmt::try_init();
        let addr = format!("localhost:{}", LISTENER_PORT)
            .to_socket_addrs()
            .unwrap()
            .collect::<Vec<_>>()[0];

        let _ = spawn_threads(addr, 1).unwrap();

        let mut socket = TcpStream::connect(addr).unwrap();
        let pkt = b"hello world";
        socket.write_all(pkt).unwrap();
        let mut buf = [0u8; 1024];
        let mut buf = &mut buf[..pkt.len()];
        socket.read_exact(&mut buf).unwrap();
        assert_eq!(buf, pkt);
    }
}
