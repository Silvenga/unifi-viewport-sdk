use std::error::Error as StdError;
use std::io;
use thiserror::Error;
use unifi_discovery::DiscoveryError;

/// Errors from the device-side Viewport.
#[derive(Debug, Error)]
pub enum DeviceError {
    /// Failed to generate or parse the self-signed TLS certificate.
    #[error("certificate error: {0}")]
    Cert(#[from] Box<dyn StdError + Send + Sync>),

    /// Failed to build the TLS server configuration.
    #[error("TLS config error: {0}")]
    Tls(#[from] rustls::Error),

    /// Failed to bind or accept on the TCP listener.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Failed to serialize or deserialize JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Failed to start or run the discovery responder.
    #[error("discovery error: {0}")]
    Discovery(#[from] DiscoveryError),

    /// The adoption request was rejected (wrong password, missing fields, etc.).
    #[error("adoption rejected: {0}")]
    Rejected(String),
}
