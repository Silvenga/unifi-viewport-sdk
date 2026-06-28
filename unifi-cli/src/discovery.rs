use clap::{Args, Subcommand};
use std::net::Ipv4Addr;
use std::process::ExitCode;
use std::time::Duration;
use tracing::{info, warn};
use unifi_discovery::DiscoveryClient;

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
}

pub async fn run(command: DiscoveryCommand, args: &DiscoveryArgs) -> ExitCode {
    match command {
        DiscoveryCommand::Query {
            response_timeout_secs,
        } => run_query(args, response_timeout_secs).await,
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
