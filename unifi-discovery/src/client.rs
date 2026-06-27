use crate::error::DiscoveryError;
use crate::message::DiscoveryMessage;
use crate::types::{DeviceInfo, MacAddress};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time;
use tracing::{debug, warn};

const DEFAULT_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(233, 89, 188, 1);
const DEFAULT_PORT: u16 = 10001;
const DEFAULT_BROADCAST_ADDR: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);
const DEFAULT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

/// Used to discover UniFi (Protect) devices on the network.
#[derive(Debug, Clone)]
pub struct DiscoveryClient {
    multicast_addr: Ipv4Addr,
    broadcast_addr: Ipv4Addr,
    port: u16,
    response_timeout: Duration,
    bind_addr: Ipv4Addr,
}

impl Default for DiscoveryClient {
    fn default() -> Self {
        Self {
            multicast_addr: DEFAULT_MULTICAST_ADDR,
            broadcast_addr: DEFAULT_BROADCAST_ADDR,
            port: DEFAULT_PORT,
            response_timeout: DEFAULT_RESPONSE_TIMEOUT,
            bind_addr: Ipv4Addr::UNSPECIFIED,
        }
    }
}

impl DiscoveryClient {
    /// Creates a client with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the multicast group address for discovery queries (default: `233.89.188.1`).
    pub fn with_multicast_addr(mut self, addr: Ipv4Addr) -> Self {
        self.multicast_addr = addr;
        self
    }

    /// Sets the broadcast address for discovery queries (default: `255.255.255.255`).
    pub fn with_broadcast_addr(mut self, addr: Ipv4Addr) -> Self {
        self.broadcast_addr = addr;
        self
    }

    /// Sets the UDP port for discovery queries (default: `10001`).
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets how long to wait for responses after each query (default: 2s).
    pub fn with_response_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = timeout;
        self
    }

    /// Sets the local address to bind the UDP socket to (default: `0.0.0.0`).
    pub fn with_bind_addr(mut self, addr: Ipv4Addr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Sends a discovery query and collects device responses.
    ///
    /// Sends a `CMD_INFO` query to both the multicast group and the broadcast
    /// address, then listens for `response_timeout` for device TLV responses.
    /// Returns all devices discovered.
    pub async fn query(&self) -> Result<Vec<DeviceInfo>, DiscoveryError> {
        let bind = SocketAddrV4::new(self.bind_addr, 0);
        let sock = UdpSocket::bind(bind).await?;
        sock.set_broadcast(true)?;

        let multicast_target = SocketAddrV4::new(self.multicast_addr, self.port);
        let broadcast_target = SocketAddrV4::new(self.broadcast_addr, self.port);

        debug!("Sending discovery query");

        let query_payload = DiscoveryMessage::InfoQuery.encode();

        debug!(target = %multicast_target, "Sending multicast query");
        sock.send_to(&query_payload, multicast_target).await?;

        debug!(target = %broadcast_target, "Sending broadcast query");
        sock.send_to(&query_payload, broadcast_target).await?;

        let devices = self.collect_responses(&sock).await;
        let mut seen: HashMap<MacAddress, DeviceInfo> = HashMap::new();
        for device in devices {
            if let Ok(Some(mac)) = device.get_mac() {
                seen.entry(mac).or_insert(device);
            }
        }
        let mut devices: Vec<_> = seen.into_values().collect();
        devices.sort_by_key(|d| d.get_ip().ok().flatten());
        Ok(devices)
    }

    async fn collect_responses(&self, sock: &UdpSocket) -> Vec<DeviceInfo> {
        let mut devices = Vec::new();
        let deadline = time::Instant::now() + self.response_timeout;
        let mut buf = vec![0u8; 4096];

        loop {
            let remaining = deadline.saturating_duration_since(time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match time::timeout(remaining, sock.recv_from(&mut buf)).await {
                Ok(Ok((n, src))) => {
                    debug!(from = %src, len = n, "Received response");

                    match DiscoveryMessage::try_from(&buf[..n]) {
                        Ok(DiscoveryMessage::InfoResponse(info)) => {
                            debug!(tlv_count = info.tlvs().len(), "Parsed discovery response");
                            debug!(
                                mac = ?info.get_mac(),
                                ip = ?info.get_ip(),
                                hostname = info.get_hostname().as_deref().unwrap_or("?"),
                                is_default = info.get_is_default().ok().flatten().unwrap_or(false),
                                "Discovered device"
                            );
                            devices.push(info);
                        }
                        Ok(DiscoveryMessage::InfoQuery) => {
                            debug!(from = %src, "Received query instead of response, ignoring");
                        }
                        Err(e) => {
                            warn!(from = %src, error = %e, "Failed to parse response");
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!(error = %e, "Receive error");
                }
                Err(_) => {
                    debug!("Response timeout elapsed");
                    break;
                }
            }
        }

        devices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_default_then_uses_spec_multicast_and_port() {
        let client = DiscoveryClient::new();
        assert_eq!(client.multicast_addr, Ipv4Addr::new(233, 89, 188, 1));
        assert_eq!(client.port, 10001);
        assert_eq!(client.broadcast_addr, Ipv4Addr::new(255, 255, 255, 255));
    }

    #[test]
    fn when_with_port_then_port_updated() {
        let client = DiscoveryClient::new().with_port(20000);
        assert_eq!(client.port, 20000);
    }

    #[test]
    fn when_with_multicast_addr_then_addr_updated() {
        let client = DiscoveryClient::new().with_multicast_addr(Ipv4Addr::new(224, 0, 0, 1));
        assert_eq!(client.multicast_addr, Ipv4Addr::new(224, 0, 0, 1));
    }

    #[test]
    fn when_with_broadcast_addr_then_addr_updated() {
        let client = DiscoveryClient::new().with_broadcast_addr(Ipv4Addr::new(10, 0, 0, 255));
        assert_eq!(client.broadcast_addr, Ipv4Addr::new(10, 0, 0, 255));
    }

    #[test]
    fn when_with_response_timeout_then_timeout_updated() {
        let client = DiscoveryClient::new().with_response_timeout(Duration::from_secs(5));
        assert_eq!(client.response_timeout, Duration::from_secs(5));
    }

    #[test]
    fn when_with_bind_addr_then_bind_updated() {
        let client = DiscoveryClient::new().with_bind_addr(Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(client.bind_addr, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn when_builder_chain_then_all_fields_set() {
        let client = DiscoveryClient::new()
            .with_port(12345)
            .with_multicast_addr(Ipv4Addr::new(224, 0, 0, 1))
            .with_broadcast_addr(Ipv4Addr::new(10, 0, 0, 255))
            .with_response_timeout(Duration::from_secs(3))
            .with_bind_addr(Ipv4Addr::new(10, 0, 0, 1));

        assert_eq!(client.port, 12345);
        assert_eq!(client.multicast_addr, Ipv4Addr::new(224, 0, 0, 1));
        assert_eq!(client.broadcast_addr, Ipv4Addr::new(10, 0, 0, 255));
        assert_eq!(client.response_timeout, Duration::from_secs(3));
        assert_eq!(client.bind_addr, Ipv4Addr::new(10, 0, 0, 1));
    }
}
