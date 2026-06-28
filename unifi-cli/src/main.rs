//! CLI tool for UniFi Protect protocol testing and debugging.

mod cli;
mod device;
mod discovery;
use cli::{parse_args, Command};
use std::process::ExitCode;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = parse_args();

    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    match cli.command {
        Command::Discovery { command, args } => discovery::run(command, &args).await,
        Command::Device { command } => device::run(command).await,
    }
}
