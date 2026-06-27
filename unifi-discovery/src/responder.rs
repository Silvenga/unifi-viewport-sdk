use crate::error::DiscoveryError;
use crate::message::DiscoveryMessage;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

const DEFAULT_PORT: u16 = 10001;
const DEFAULT_BIND_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const DEFAULT_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(233, 89, 188, 1);

/// Builder for the device-side discovery responder.
///
/// Listens on a UDP port for `CMD_INFO` queries and responds by calling the
/// provided closure to produce a fresh [`DiscoveryMessage`] for each query.
/// This allows dynamic fields like uptime to be updated on each response.
///
/// Defaults to binding on port `10001` (the protocol standard), but the port
/// can be overridden for testing.
pub struct DiscoveryResponder<F>
where
    F: Fn() -> DiscoveryMessage + Send + Sync,
{
    port: u16,
    bind_addr: Ipv4Addr,
    multicast_addr: Ipv4Addr,
    responder: Arc<F>,
}

impl<F> DiscoveryResponder<F>
where
    F: Fn() -> DiscoveryMessage + Send + Sync,
{
    /// Creates a responder with the given response factory closure.
    pub fn new(responder: F) -> Self {
        Self {
            port: DEFAULT_PORT,
            bind_addr: DEFAULT_BIND_ADDR,
            multicast_addr: DEFAULT_MULTICAST_ADDR,
            responder: Arc::new(responder),
        }
    }

    /// Sets the UDP port to listen on (default: `10001`).
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the local address to bind to (default: `0.0.0.0`).
    pub fn with_bind_addr(mut self, addr: Ipv4Addr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Sets the multicast group to join (default: `233.89.188.1`).
    pub fn with_multicast_addr(mut self, addr: Ipv4Addr) -> Self {
        self.multicast_addr = addr;
        self
    }

    /// Listens for discovery queries and responds with device info.
    ///
    /// Joins the configured multicast group so queries sent to both
    /// multicast and broadcast are received. Runs until the future is dropped.
    pub async fn listen(&self) -> Result<(), DiscoveryError> {
        let bind = SocketAddrV4::new(self.bind_addr, self.port);
        let sock = UdpSocket::bind(bind).await?;
        sock.join_multicast_v4(self.multicast_addr, self.bind_addr)?;
        info!(
            bind_addr = %sock.local_addr()?,
            multicast = %self.multicast_addr,
            "Discovery responder listening"
        );

        let mut buf = vec![0u8; 4096];

        loop {
            let (n, src) = sock.recv_from(&mut buf).await?;
            debug!(from = %src, len = n, "Received query");

            match DiscoveryMessage::try_from(&buf[..n]) {
                Ok(DiscoveryMessage::InfoQuery) => {
                    debug!(from = %src, "Valid CMD_INFO query, responding");
                    let response = (self.responder)().encode();
                    sock.send_to(&response, src).await?;
                }
                Ok(DiscoveryMessage::InfoResponse(_)) => {
                    warn!(from = %src, "Received response instead of query, ignoring");
                }
                Err(e) => {
                    warn!(from = %src, error = %e, "Invalid query, ignoring");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::Frame;
    use crate::types::DeviceInfo;
    use std::time::Duration;

    #[test]
    fn when_responder_encodes_then_roundtrips_through_parse() {
        let mut info = DeviceInfo::new();
        info.set_mac([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        info.set_ip(
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            Ipv4Addr::new(10, 0, 0, 5),
        );
        info.set_firmware("1.0.0");
        info.set_uptime(Duration::from_secs(300));
        info.set_hostname("Test");
        info.set_platform("Test");
        info.set_is_default(true);
        info.set_guid("550e8400-e29b-41d4-a716-446655440000");

        let responder =
            DiscoveryResponder::new(move || DiscoveryMessage::InfoResponse(info.clone()));
        let response = (responder.responder)();
        let payload = response.encode();

        let frame = Frame::parse(&payload).unwrap();
        let decoded = DeviceInfo::from_tlvs(frame.values.clone());

        assert_eq!(
            decoded.get_mac().unwrap().unwrap(),
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
        );
        assert_eq!(
            decoded.get_ip().unwrap().unwrap(),
            Ipv4Addr::new(10, 0, 0, 5)
        );
        assert_eq!(decoded.get_firmware().unwrap(), "1.0.0");
        assert_eq!(
            decoded.get_uptime().unwrap().unwrap(),
            Duration::from_secs(300)
        );
        assert_eq!(decoded.get_hostname().unwrap(), "Test");
        assert_eq!(decoded.get_platform().unwrap(), "Test");
        assert!(decoded.get_is_default().unwrap().unwrap());
        assert_eq!(
            decoded.get_guid().unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
