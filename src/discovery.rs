use std::net::Ipv4Addr;
use std::time::Duration;
use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

const SERVICE_TYPE: &str = "_lancp._tcp.local.";

#[derive(Debug, Clone)]
pub struct Host {
    pub addr: std::net::IpAddr,
    pub name: String,
    pub port: u16,
}

impl std::fmt::Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.addr)
    }
}

/// Returns non-loopback local IPv4 addresses.
pub fn local_ipv4_addrs() -> Vec<Ipv4Addr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| {
            if let if_addrs::IfAddr::V4(v4) = iface.addr {
                if !v4.ip.is_loopback() { Some(v4.ip) } else { None }
            } else {
                None
            }
        })
        .collect()
}

/// Register this machine as a lancp receiver via mDNS. Runs until cancelled.
pub async fn announce(data_port: u16) -> Result<()> {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "lancp-host".to_string());

    let local_ips = local_ipv4_addrs();
    anyhow::ensure!(!local_ips.is_empty(), "No local IPv4 addresses found");

    let host_name = format!("{}.local.", hostname);
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &hostname,
        &host_name,
        std::net::IpAddr::V4(local_ips[0]),
        data_port,
        None,
    )?;

    let mdns = ServiceDaemon::new()?;
    mdns.register(service)?;

    // Keep daemon alive until the task is cancelled.
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}

/// Browse for lancp receivers on the LAN for `timeout_secs`, then return all found.
pub async fn discover(timeout_secs: u64) -> Result<Vec<Host>> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse(SERVICE_TYPE)?;

    let mut hosts: Vec<Host> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, receiver.recv_async()).await {
            Ok(Ok(ServiceEvent::ServiceResolved(info))) => {
                if let Some(addr) = info.get_addresses_v4().into_iter().next() {
                    let ip = std::net::IpAddr::V4(*addr);
                    if !hosts.iter().any(|h| h.addr == ip) {
                        let name = info.get_hostname().trim_end_matches('.').to_string();
                        hosts.push(Host { addr: ip, name, port: info.get_port() });
                    }
                }
            }
            Ok(Ok(_)) => {}   // SearchStarted, ServiceFound, etc.
            Ok(Err(_)) | Err(_) => break,
        }
    }

    mdns.stop_browse(SERVICE_TYPE).ok();
    Ok(hosts)
}
