use clap::{Args, Subcommand};
use local_ip_address::local_ip;
use std::net::{IpAddr, Ipv4Addr};
use std::process::ExitCode;
use std::time::Duration;
use sysinfo::{Networks, System};
use tokio::time::timeout;
use tracing::{info, warn};
use unifi_discovery::{DeviceInfo, DiscoveryClient, DiscoveryMessage, DiscoveryResponder};

#[derive(Args)]
pub struct DiscoveryArgs {
    /// Multicast group address for discovery queries.
    #[arg(
        long,
        env = "UNIFI_DISCOVERY_MULTICAST",
        default_value = "233.89.188.1",
        global = true
    )]
    pub multicast_addr: Ipv4Addr,

    /// Broadcast address for discovery queries.
    #[arg(
        long,
        env = "UNIFI_DISCOVERY_BROADCAST",
        default_value = "255.255.255.255",
        global = true
    )]
    pub broadcast_addr: Ipv4Addr,

    /// UDP port for discovery.
    #[arg(
        long,
        env = "UNIFI_DISCOVERY_PORT",
        default_value_t = 10001,
        global = true
    )]
    pub port: u16,

    /// Local address to bind the UDP socket to.
    #[arg(
        long,
        env = "UNIFI_DISCOVERY_BIND",
        default_value = "0.0.0.0",
        global = true
    )]
    pub bind_addr: Ipv4Addr,
}

#[derive(Subcommand)]
pub enum DiscoveryCommand {
    /// Send a discovery query and print discovered devices.
    Query {
        /// Seconds to wait for responses after the query.
        #[arg(long, default_value_t = 2)]
        response_timeout_secs: u64,
    },
    /// Listen for incoming discovery queries and respond as a simulated device.
    Listen {
        /// MAC address to advertise (e.g. "AA:BB:CC:DD:EE:FF").
        /// Defaults to the MAC of the default-route interface.
        #[arg(long, env = "UNIFI_DISCOVERY_MAC")]
        mac: Option<String>,

        /// IP address to advertise. Defaults to the IP of the default-route interface.
        #[arg(long, env = "UNIFI_DISCOVERY_IP")]
        ip: Option<Ipv4Addr>,

        /// Hostname to advertise. Defaults to the system hostname.
        #[arg(long, env = "UNIFI_DISCOVERY_HOSTNAME")]
        hostname: Option<String>,

        /// Platform/model to advertise.
        #[arg(long, env = "UNIFI_DISCOVERY_PLATFORM", default_value = "UP Viewport")]
        platform: String,

        /// Whether the device appears as adoptable (factory default).
        /// When false (default), the device shows as already adopted.
        #[arg(long, env = "UNIFI_DISCOVERY_ADOPTABLE", default_value_t = false)]
        adoptable: bool,

        /// Stop listening after N seconds. 0 means listen forever.
        #[arg(long, env = "UNIFI_DISCOVERY_TIMEOUT", default_value_t = 0)]
        timeout_secs: u64,
    },
}

pub async fn run(command: DiscoveryCommand, args: &DiscoveryArgs) -> ExitCode {
    match command {
        DiscoveryCommand::Query {
            response_timeout_secs,
        } => run_query(args, response_timeout_secs).await,
        DiscoveryCommand::Listen {
            mac,
            ip,
            hostname,
            platform,
            adoptable,
            timeout_secs,
        } => run_listen(args, mac, ip, hostname, &platform, adoptable, timeout_secs).await,
    }
}

async fn run_query(args: &DiscoveryArgs, response_timeout_secs: u64) -> ExitCode {
    let client = DiscoveryClient::new()
        .with_multicast_addr(args.multicast_addr)
        .with_broadcast_addr(args.broadcast_addr)
        .with_port(args.port)
        .with_bind_addr(args.bind_addr)
        .with_response_timeout(Duration::from_secs(response_timeout_secs));

    match client.query().await {
        Ok(devices) => {
            if devices.is_empty() {
                info!("No devices discovered");
            } else {
                info!(count = devices.len(), "Discovery complete");
                for device in &devices {
                    let mac = device.get_mac().ok().flatten().unwrap_or([0; 6]);
                    info!(
                        mac = %format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                        ),
                        ip = ?device.get_ip().ok().flatten(),
                        hostname = device.get_hostname().as_deref().unwrap_or("?"),
                        platform = device.get_platform().as_deref().unwrap_or("?"),
                        is_default = device.get_is_default().ok().flatten().unwrap_or(false),
                        "Discovered device"
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            warn!(error = %e, "Discovery failed");
            ExitCode::FAILURE
        }
    }
}

async fn run_listen(
    args: &DiscoveryArgs,
    mac: Option<String>,
    ip: Option<Ipv4Addr>,
    hostname: Option<String>,
    platform: &str,
    adoptable: bool,
    timeout_secs: u64,
) -> ExitCode {
    let detected = detect_interface();

    let mac_bytes = match mac {
        Some(m) => match parse_mac(&m) {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(error = %e, "Invalid MAC address");
                return ExitCode::FAILURE;
            }
        },
        None => match detected.mac {
            Some(m) => m,
            None => {
                warn!("No MAC address specified and no suitable network interface found");
                return ExitCode::FAILURE;
            }
        },
    };

    let ip = ip.or(detected.ip);
    let hostname = hostname
        .or(detected.hostname)
        .unwrap_or_else(|| "unifi-cli".to_owned());

    info!(
        mac = %format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac_bytes[0], mac_bytes[1], mac_bytes[2], mac_bytes[3], mac_bytes[4], mac_bytes[5]),
        ip = ?ip,
        hostname = %hostname,
        platform,
        timeout = if timeout_secs > 0 { format!("{timeout_secs}s") } else { "forever".to_owned() },
        "Starting discovery responder"
    );

    let platform = platform.to_owned();

    let responder = DiscoveryResponder::new(move || {
        let mut info = DeviceInfo::new();
        info.set_mac(mac_bytes);
        if let Some(ip) = ip {
            info.set_ip(mac_bytes, ip);
        }
        info.set_hostname(&hostname);
        info.set_platform(&platform);
        info.set_uptime(Duration::from_secs(System::uptime()));
        info.set_is_default(adoptable);
        DiscoveryMessage::InfoResponse(info)
    })
    .with_port(args.port)
    .with_bind_addr(args.bind_addr)
    .with_multicast_addr(args.multicast_addr);

    let listen_fut = responder.listen();

    if timeout_secs > 0 {
        match timeout(Duration::from_secs(timeout_secs), listen_fut).await {
            Ok(Ok(())) => {
                info!("Responder stopped");
                ExitCode::SUCCESS
            }
            Ok(Err(e)) => {
                warn!(error = %e, "Responder failed");
                ExitCode::FAILURE
            }
            Err(_) => {
                info!(timeout_secs, "Listen timeout elapsed, stopping");
                ExitCode::SUCCESS
            }
        }
    } else {
        match listen_fut.await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                warn!(error = %e, "Responder failed");
                ExitCode::FAILURE
            }
        }
    }
}

struct DetectedInterface {
    mac: Option<[u8; 6]>,
    ip: Option<Ipv4Addr>,
    hostname: Option<String>,
}

fn detect_interface() -> DetectedInterface {
    let default_ip = local_ip().ok();
    let networks = Networks::new_with_refreshed_list();

    let mut mac: Option<[u8; 6]> = None;
    let mut ip: Option<Ipv4Addr> = None;

    if let Some(default_ip) = default_ip {
        for (_, data) in &networks {
            for ip_net in data.ip_networks() {
                if ip_net.addr == default_ip {
                    if let IpAddr::V4(v4) = default_ip {
                        ip = Some(v4);
                    }
                    if let Ok(bytes) = parse_mac(&data.mac_address().to_string()) {
                        if bytes != [0; 6] {
                            mac = Some(bytes);
                        }
                    }
                    break;
                }
            }
        }
    }

    if ip.is_none() {
        for (_, data) in &networks {
            if ip.is_none() {
                for ip_net in data.ip_networks() {
                    if let IpAddr::V4(v4) = ip_net.addr {
                        if !v4.is_loopback() && !v4.is_unspecified() {
                            ip = Some(v4);
                            break;
                        }
                    }
                }
            }
        }
    }

    if mac.is_none() {
        for (_, data) in &networks {
            if mac.is_some() {
                break;
            }
            if let Ok(bytes) = parse_mac(&data.mac_address().to_string()) {
                if bytes != [0; 6] {
                    mac = Some(bytes);
                }
            }
        }
    }

    let hostname = System::host_name();

    DetectedInterface { mac, ip, hostname }
}

fn parse_mac(s: &str) -> Result<[u8; 6], String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err(format!(
            "Expected 6 hex octets separated by ':', got {parts:?}"
        ));
    }
    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] =
            u8::from_str_radix(part, 16).map_err(|e| format!("Invalid hex byte '{part}': {e}"))?;
    }
    Ok(mac)
}
