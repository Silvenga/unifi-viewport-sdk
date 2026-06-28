use crate::error::DeviceError;
use crate::routes::AdoptionRequest;
use crate::server::DeviceServer;
use crate::state::DeviceState;
use crate::storage::{DeviceStorage, InMemoryStorage, SharedStorage};
use rcgen::{CertificateParams, DnType, ExtendedKeyUsagePurpose, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::error::Error as StdError;
use std::net::Ipv4Addr;
use std::str;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::select;
use tracing::{debug, info, warn};
use unifi_discovery::{DeviceInfo, DiscoveryMessage, DiscoveryResponder, MacAddress};

const DEFAULT_DISCOVERY_PORT: u16 = 10001;
const DEFAULT_ADOPTION_PORT: u16 = 8080;
const DEFAULT_BIND_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const DEFAULT_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(233, 89, 188, 1);
const DEFAULT_PLATFORM: &str = "UP Viewport";
const DEFAULT_FIRMWARE: &str = "UPV.qcs605.v1.4.33.0.4698daf26.260416.1114";
const DEFAULT_ANONYMOUS_ID: &str = "";

/// The central struct for a simulated Viewport device.
///
/// Orchestrates both the UDP discovery responder (port 10001) and the
/// TLS adoption server (port 8080). The device is stateful: it starts
/// in factory-default state (adoptable) and transitions to adopted
/// when the controller sends a valid adoption request.
///
/// State is persisted via the consumer-provided [`DeviceStorage`] impl,
/// allowing the device to restore its adopted state across restarts.
pub struct ViewPortDevice {
    mac: MacAddress,
    ip: Option<Ipv4Addr>,
    hostname: String,
    platform: String,
    firmware: String,
    anonymous_id: String,
    discovery_port: u16,
    adoption_port: u16,
    bind_addr: Ipv4Addr,
    multicast_addr: Ipv4Addr,
    storage: SharedStorage,
    uptime_provider: Arc<dyn Fn() -> Duration + Send + Sync>,
}

/// Builder for [`ViewPortDevice`].
pub struct ViewPortDeviceBuilder {
    mac: Option<MacAddress>,
    ip: Option<Ipv4Addr>,
    hostname: String,
    platform: String,
    firmware: String,
    anonymous_id: String,
    storage: SharedStorage,
    discovery_port: u16,
    adoption_port: u16,
    bind_addr: Ipv4Addr,
    multicast_addr: Ipv4Addr,
    uptime_provider: Arc<dyn Fn() -> Duration + Send + Sync>,
}

impl ViewPortDeviceBuilder {
    /// Sets the device MAC address (required).
    pub fn mac(mut self, mac: MacAddress) -> Self {
        self.mac = Some(mac);
        self
    }

    /// Sets the device IP address. If set, included in discovery responses.
    pub fn ip(mut self, ip: Ipv4Addr) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Sets the hostname advertised in discovery responses (default: `"UP Viewport"`).
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    /// Sets the platform/model string (default: `"UP Viewport"`).
    pub fn platform(mut self, platform: impl Into<String>) -> Self {
        self.platform = platform.into();
        self
    }

    /// Sets the firmware version string (default: empty).
    pub fn firmware(mut self, firmware: impl Into<String>) -> Self {
        self.firmware = firmware.into();
        self
    }

    /// Sets the anonymous ID (UUID string, TLV type `0x20`).
    pub fn anonymous_id(mut self, anonymous_id: impl Into<String>) -> Self {
        self.anonymous_id = anonymous_id.into();
        self
    }

    /// Sets the persistent storage impl (default: [`InMemoryStorage`]).
    pub fn storage(mut self, storage: impl DeviceStorage + 'static) -> Self {
        self.storage = Arc::new(storage);
        self
    }

    /// Sets the UDP port for discovery (default: `10001`).
    pub fn discovery_port(mut self, port: u16) -> Self {
        self.discovery_port = port;
        self
    }

    /// Sets the TCP port for the adoption server (default: `8080`).
    pub fn adoption_port(mut self, port: u16) -> Self {
        self.adoption_port = port;
        self
    }

    /// Sets the bind address (default: `0.0.0.0`).
    pub fn bind_addr(mut self, addr: Ipv4Addr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Sets the multicast group for discovery (default: `233.89.188.1`).
    pub fn multicast_addr(mut self, addr: Ipv4Addr) -> Self {
        self.multicast_addr = addr;
        self
    }

    /// Sets the uptime provider closure, called on each discovery query
    /// to produce the current device uptime (default: returns 0).
    pub fn uptime_provider<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Duration + Send + Sync + 'static,
    {
        self.uptime_provider = Arc::new(f);
        self
    }

    /// Builds the [`ViewPortDevice`], loading persisted state from storage.
    pub fn build(self) -> Result<ViewPortDevice, DeviceError> {
        let mac = self
            .mac
            .ok_or_else(|| DeviceError::Rejected("MAC address is required".to_owned()))?;

        Ok(ViewPortDevice {
            mac,
            ip: self.ip,
            hostname: self.hostname,
            platform: self.platform,
            firmware: self.firmware,
            anonymous_id: self.anonymous_id,
            discovery_port: self.discovery_port,
            adoption_port: self.adoption_port,
            bind_addr: self.bind_addr,
            multicast_addr: self.multicast_addr,
            storage: self.storage,
            uptime_provider: self.uptime_provider,
        })
    }
}

impl ViewPortDevice {
    /// Creates a new builder with defaults.
    pub fn builder() -> ViewPortDeviceBuilder {
        ViewPortDeviceBuilder {
            mac: None,
            ip: None,
            hostname: DEFAULT_PLATFORM.to_owned(),
            platform: DEFAULT_PLATFORM.to_owned(),
            firmware: DEFAULT_FIRMWARE.to_owned(),
            anonymous_id: DEFAULT_ANONYMOUS_ID.to_owned(),
            storage: Arc::new(InMemoryStorage::new()),
            discovery_port: DEFAULT_DISCOVERY_PORT,
            adoption_port: DEFAULT_ADOPTION_PORT,
            bind_addr: DEFAULT_BIND_ADDR,
            multicast_addr: DEFAULT_MULTICAST_ADDR,
            uptime_provider: Arc::new(|| Duration::from_secs(0)),
        }
    }

    /// Starts the device: runs the discovery responder and adoption server
    /// concurrently until both complete or an error occurs.
    ///
    /// On a valid adoption request, the device:
    /// 1. Generates a client certificate (for the UCP4 WebSocket connection).
    /// 2. Stores the controller's console ID and name.
    /// 3. Marks itself as adopted (`is_default = 0x00` in discovery responses).
    /// 4. Persists the new state via [`DeviceStorage`].
    pub async fn listen(self) -> Result<(), DeviceError> {
        let state = self
            .storage
            .load()
            .map_err(DeviceError::Rejected)?
            .unwrap_or_else(DeviceState::factory_default);

        let state = Arc::new(Mutex::new(state));
        let storage = self.storage.clone();

        let discovery = self.build_discovery_responder(state.clone())?;
        let adoption = self.build_adoption_server(state.clone(), storage)?;

        info!(
            mac = %format!(
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                self.mac[0], self.mac[1], self.mac[2], self.mac[3], self.mac[4], self.mac[5]
            ),
            ip = ?self.ip,
            hostname = %self.hostname,
            adopted = state.lock().unwrap().is_adopted,
            "ViewPort device starting"
        );

        let discovery_fut = discovery.listen();
        let adoption_fut = adoption.serve();

        select! {
            result = discovery_fut => {
                warn!("Discovery responder stopped");
                result?;
            }
            result = adoption_fut => {
                warn!("Adoption server stopped");
                result?;
            }
        }

        Ok(())
    }

    fn build_discovery_responder(
        &self,
        state: Arc<Mutex<DeviceState>>,
    ) -> Result<DiscoveryResponder<impl Fn() -> DiscoveryMessage + Send + Sync>, DeviceError> {
        let mac = self.mac;
        let ip = self.ip;
        let hostname = self.hostname.clone();
        let platform = self.platform.clone();
        let firmware = self.firmware.clone();
        let anonymous_id = self.anonymous_id.clone();
        let uptime_provider = self.uptime_provider.clone();

        let responder = move || {
            let mut info = DeviceInfo::new();
            info.set_mac(mac);
            if let Some(ip) = ip {
                info.set_ip(mac, ip);
            }
            info.set_hostname(&hostname);
            info.set_platform(&platform);
            if !firmware.is_empty() {
                info.set_firmware(&firmware);
            }
            info.set_uptime(uptime_provider());

            if !anonymous_id.is_empty() {
                info.set_anonymous_id(&anonymous_id);
            }

            let state = state.lock().unwrap();
            info.set_is_default(state.is_adoptable());

            if let Some(id) = state.controller_id_binary {
                info.set_controller_id(id);
            }

            info.set_guid(state.guid);

            drop(state);

            DiscoveryMessage::InfoResponse(info)
        };

        Ok(DiscoveryResponder::new(responder)
            .with_port(self.discovery_port)
            .with_bind_addr(self.bind_addr)
            .with_multicast_addr(self.multicast_addr))
    }

    fn build_adoption_server(
        &self,
        state: Arc<Mutex<DeviceState>>,
        storage: SharedStorage,
    ) -> Result<DeviceServer, DeviceError> {
        let expected_password = state.lock().unwrap().password.clone();

        let callback = move |req: AdoptionRequest| {
            debug!(
                controller = %req.controller,
                nvr = %req.nvr,
                console_id = %req.console_id,
                "Processing adoption request"
            );

            let (cert_der, key_der) = match generate_client_cert() {
                Ok((cert, key)) => (cert, key),
                Err(e) => {
                    warn!(error = %e, "Failed to generate client certificate");
                    return Err("certificate generation failed".to_owned());
                }
            };

            let controller_id_binary = parse_console_id(&req.console_id);

            let mut s = state.lock().unwrap();
            s.is_adopted = true;
            s.controller_id = Some(req.console_id.clone());
            s.controller_name = Some(req.console_name.clone());
            s.client_cert_der = Some(cert_der.to_vec());
            s.client_key_der = Some(key_der.secret_der().to_vec());
            s.controller_id_binary = controller_id_binary;

            if let Err(e) = storage.save(&s) {
                warn!(error = %e, "Failed to persist device state");
                return Err("storage failed".to_owned());
            }

            info!(
                controller = %req.controller,
                console_name = %req.console_name,
                "Device adopted"
            );

            Ok(())
        };

        Ok(DeviceServer::new(callback)?
            .with_port(self.adoption_port)
            .with_bind_addr(self.bind_addr)
            .with_password(expected_password)
            .with_firmware(&self.firmware)
            .with_mac(self.mac))
    }
}

fn generate_client_cert(
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), Box<dyn StdError + Send + Sync>> {
    let mut params = CertificateParams::new(vec!["localhost".to_owned()])?;
    params
        .distinguished_name
        .push(DnType::CommonName, "viewport-client");
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ClientAuth);

    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + time::Duration::days(365 * 10);

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    Ok((cert_der, key_der))
}

fn parse_console_id(console_id: &str) -> Option<[u8; 16]> {
    let hex: String = console_id.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        warn!(
            console_id = %console_id,
            len = hex.len(),
            "Console ID did not parse to 16 bytes"
        );
        return None;
    }
    let bytes = hex.as_bytes();
    let mut arr = [0u8; 16];
    for (i, chunk) in bytes.chunks(2).enumerate() {
        arr[i] = u8::from_str_radix(str::from_utf8(chunk).unwrap_or("00"), 16).ok()?;
    }
    Some(arr)
}
