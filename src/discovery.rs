use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use anyhow::Result;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

/// Administratively-scoped multicast address (site-local, never forwarded by routers).
const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 42, 98);

#[derive(Debug, Clone)]
pub struct Host {
    pub addr: std::net::IpAddr,
    pub name: String,
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.addr)
    }
}

/// Broadcast presence as a receiver on `port`. Runs until cancelled.
pub async fn announce(port: u16) -> Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;
    socket.set_multicast_ttl_v4(1)?; // link-local only
    let dest = SocketAddr::new(MULTICAST_ADDR.into(), port);
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());
    let msg = format!("LANCP_RCV:{}", hostname);
    loop {
        socket.send_to(msg.as_bytes(), dest).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Listen for receiver announcements for `timeout_secs` and return discovered hosts.
pub async fn discover(port: u16, timeout_secs: u64) -> Result<Vec<Host>> {
    let socket = make_multicast_socket(port)?;
    socket.join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)?;

    let mut hosts: Vec<Host> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    let mut buf = [0u8; 256];
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, addr))) => {
                if let Ok(msg) = std::str::from_utf8(&buf[..len]) {
                    if let Some(name) = msg.strip_prefix("LANCP_RCV:") {
                        let ip = addr.ip();
                        if !hosts.iter().any(|h| h.addr == ip) {
                            hosts.push(Host { addr: ip, name: name.to_string() });
                        }
                    }
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => break,
        }
    }
    Ok(hosts)
}

/// Create a UDP socket with SO_REUSEADDR bound to `port`, ready for multicast.
fn make_multicast_socket(port: u16) -> Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port).into())?;
    Ok(UdpSocket::from_std(socket.into())?)
}
