use clap::{Args, Subcommand};
use local_ip_address::local_ip;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::process::ExitCode;
use std::time::Duration;
use sysinfo::{Networks, System};
use tokio::time::timeout;
use tracing::{info, warn};
use unifi_device_viewport::{InMemoryStorage, ViewPortDevice};

#[derive(Subcommand)]
pub enum DeviceCommand {
    /// Start a simulated Viewport device (discovery + adoption server)
    Viewport {
        #[command(flatten)]
        args: ViewportArgs,
    },
}

#[derive(Args)]
pub struct ViewportArgs {
    /// MAC address to advertise (e.g. "AA:BB:CC:DD:EE:FF").
    /// Defaults to the MAC of the default-route interface.
    #[arg(long, env = "UNIFI_DEVICE_MAC")]
    mac: Option<String>,

    /// IP address to advertise. Defaults to the IP of the default-route interface.
    #[arg(long, env = "UNIFI_DEVICE_IP")]
    ip: Option<Ipv4Addr>,

    /// Hostname to advertise. Defaults to the system hostname.
    #[arg(long, env = "UNIFI_DEVICE_HOSTNAME")]
    hostname: Option<String>,

    /// Platform/model to advertise.
    #[arg(long, env = "UNIFI_DEVICE_PLATFORM", default_value = "UP Viewport")]
    platform: String,

    /// Firmware version to advertise.
    #[arg(
        long,
        env = "UNIFI_DEVICE_FIRMWARE",
        default_value = "UPV.qcs605.v1.4.33.0.4698daf26.260416.1114"
    )]
    firmware: String,

    /// Anonymous ID (UUID string).
    #[arg(long, env = "UNIFI_DEVICE_ANONYMOUS_ID")]
    anonymous_id: Option<String>,

    /// UDP port for discovery responses.
    #[arg(long, env = "UNIFI_DEVICE_DISCOVERY_PORT", default_value_t = 10001)]
    discovery_port: u16,

    /// TCP port for the adoption server.
    #[arg(long, env = "UNIFI_DEVICE_ADOPTION_PORT", default_value_t = 8080)]
    adoption_port: u16,

    /// Address to bind both servers to.
    #[arg(long, env = "UNIFI_DEVICE_BIND", default_value = "0.0.0.0")]
    bind_addr: Ipv4Addr,

    /// Multicast group for discovery.
    #[arg(long, env = "UNIFI_DEVICE_MULTICAST", default_value = "233.89.188.1")]
    multicast_addr: Ipv4Addr,

    /// Stop after N seconds. 0 means run forever.
    #[arg(long, env = "UNIFI_DEVICE_TIMEOUT", default_value_t = 0)]
    timeout_secs: u64,
}

pub async fn run(command: DeviceCommand) -> ExitCode {
    match command {
        DeviceCommand::Viewport { args } => run_viewport(args).await,
    }
}

async fn run_viewport(args: ViewportArgs) -> ExitCode {
    let detected = detect_interface();

    let mac_bytes = match args.mac {
        Some(ref m) => match parse_mac(m) {
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

    let ip = args.ip.or(detected.ip);
    let hostname = args
        .hostname
        .or(detected.hostname)
        .unwrap_or_else(|| "UP Viewport".to_owned());

    info!(
        mac = %format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", mac_bytes[0], mac_bytes[1], mac_bytes[2], mac_bytes[3], mac_bytes[4], mac_bytes[5]),
        ip = ?ip,
        hostname = %hostname,
        platform = %args.platform,
        firmware = %args.firmware,
        discovery_port = args.discovery_port,
        adoption_port = args.adoption_port,
        "Starting simulated Viewport device"
    );

    let mut builder = ViewPortDevice::builder()
        .mac(mac_bytes)
        .hostname(&hostname)
        .platform(&args.platform)
        .firmware(&args.firmware)
        .storage(InMemoryStorage::new())
        .discovery_port(args.discovery_port)
        .adoption_port(args.adoption_port)
        .bind_addr(args.bind_addr)
        .multicast_addr(args.multicast_addr)
        .uptime_provider(|| Duration::from_secs(System::uptime()));

    if let Some(ip) = ip {
        builder = builder.ip(ip);
    }

    if let Some(ref anonymous_id) = args.anonymous_id {
        builder = builder.anonymous_id(anonymous_id);
    }

    let device = match builder.build() {
        Ok(d) => d,
        Err(e) => {
            warn!(error = %e, "Failed to build device");
            return ExitCode::FAILURE;
        }
    };

    let listen_fut = device.listen();

    if args.timeout_secs > 0 {
        match timeout(Duration::from_secs(args.timeout_secs), listen_fut).await {
            Ok(Ok(())) => {
                info!("Device stopped");
                ExitCode::SUCCESS
            }
            Ok(Err(e)) => {
                warn!(error = %e, "Device failed");
                ExitCode::FAILURE
            }
            Err(_) => {
                info!(
                    timeout_secs = args.timeout_secs,
                    "Timeout elapsed, stopping"
                );
                ExitCode::SUCCESS
            }
        }
    } else {
        match listen_fut.await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                warn!(error = %e, "Device failed");
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
