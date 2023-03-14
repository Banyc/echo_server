use std::net::SocketAddr;

pub mod tcp_echo;
pub mod udp_echo;

pub fn domain(listen_addr: &SocketAddr) -> socket2::Domain {
    match listen_addr {
        SocketAddr::V4(_) => socket2::Domain::IPV4,
        SocketAddr::V6(_) => socket2::Domain::IPV6,
    }
}
