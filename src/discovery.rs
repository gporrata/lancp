use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use anyhow::Result;
use tokio::net::UdpSocket;

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
    socket.set_broadcast(true)?;
    let broadcast_addr = SocketAddr::new(Ipv4Addr::BROADCAST.into(), port);
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "unknown".to_string());
    let msg = format!("LANCP_RCV:{}", hostname);
    loop {
        socket.send_to(msg.as_bytes(), broadcast_addr).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Listen for receiver announcements for `timeout_secs` and return discovered hosts.
pub async fn discover(port: u16, timeout_secs: u64) -> Result<Vec<Host>> {
    let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
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
