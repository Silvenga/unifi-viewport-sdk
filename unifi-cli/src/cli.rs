use crate::discovery::{DiscoveryArgs, DiscoveryCommand};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "unifi-cli",
    about = "CLI tool for UniFi Protect protocol testing and debugging",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// UniFi Protect device discovery protocol (UDP 10001)
    Discovery {
        #[command(subcommand)]
        command: DiscoveryCommand,

        #[command(flatten)]
        args: DiscoveryArgs,
    },
}

pub fn parse_args() -> Cli {
    Parser::parse()
}
