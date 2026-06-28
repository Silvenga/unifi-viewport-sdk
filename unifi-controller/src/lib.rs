//! Controller-side implementation of the Ubiquiti Protect adoption flow.
//!
//! The controller pushes adoption info to a discovered device via HTTPS POST
//! to `https://<device>:8080/api/adopt`. The device's self-signed TLS
//! certificate is accepted without verification (no certificate pinning).
//!
//! # Example
//!
//! ```no_run
//! use unifi_controller::{adopt_viewport, AdoptionParams};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let params = AdoptionParams::new("my-token", "console-id", "UNVR")
//!     .with_hosts(vec!["192.168.0.4:7442".to_owned()])
//!     .with_nvr("UNVR4");
//!
//! let result = adopt_viewport("192.168.0.201:8080", &params).await?;
//! assert!(result.success);
//! # Ok(())
//! # }
//! ```

mod error;

pub use error::ControllerError;

use reqwest::tls::Version;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Parameters for adopting a Viewport device.
///
/// This is the payload the controller sends to the device's `/api/adopt`
/// endpoint. All fields match the captured protocol exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionParams {
    /// The device user's username (e.g. `ubnt`).
    pub username: String,

    /// The device user's password (factory default: `ubnt`).
    pub password: String,

    /// The controller's WebSocket endpoints (e.g. `["192.168.0.4:7442"]`).
    pub hosts: Vec<String>,

    /// The adoption token used to authenticate the WebSocket connection.
    pub token: String,

    /// The WebSocket protocol scheme (`"wss"`).
    pub protocol: String,

    /// Adoption mode (observed: `0`).
    pub mode: u32,

    /// The NVR model string (e.g. `"UNVR4"`).
    pub nvr: String,

    /// The controller application name (e.g. `"Protect"`).
    pub controller: String,

    /// The console's unique ID.
    pub console_id: String,

    /// The console's display name.
    pub console_name: String,
}

impl AdoptionParams {
    /// Creates adoption parameters with the given token and console info,
    /// defaulting the username/password to `ui` (factory default).
    pub fn new(
        token: impl Into<String>,
        console_id: impl Into<String>,
        console_name: impl Into<String>,
    ) -> Self {
        Self {
            username: "ubnt".to_owned(),
            password: "ubnt".to_owned(),
            hosts: vec![],
            token: token.into(),
            protocol: "wss".to_owned(),
            mode: 0,
            nvr: String::new(),
            controller: "Protect".to_owned(),
            console_id: console_id.into(),
            console_name: console_name.into(),
        }
    }

    /// Sets the device password (default: `ui`).
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    /// Sets the controller's WebSocket host endpoints.
    pub fn with_hosts(mut self, hosts: Vec<String>) -> Self {
        self.hosts = hosts;
        self
    }

    /// Sets the NVR model string.
    pub fn with_nvr(mut self, nvr: impl Into<String>) -> Self {
        self.nvr = nvr.into();
        self
    }
}

/// The response from the device after an adoption attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptionResult {
    /// Whether the adoption was accepted by the device.
    pub success: bool,
}

/// Adopts a Viewport device by sending a POST request to its `/api/adopt`
/// endpoint over HTTPS (TLS 1.3).
///
/// The device's self-signed certificate is accepted without verification
/// (matching the observed protocol behavior — no certificate pinning).
///
/// # Arguments
/// - `device_addr` - The device's IP address and port (e.g. `"192.168.0.201:8080"`).
/// - `params` - The adoption parameters to send.
pub async fn adopt_viewport(
    device_addr: &str,
    params: &AdoptionParams,
) -> Result<AdoptionResult, ControllerError> {
    let url = format!("https://{device_addr}/api/adopt");

    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .min_tls_version(Version::TLS_1_3)
        .max_tls_version(Version::TLS_1_3)
        .build()?;

    debug!(url = %url, "Sending adoption request");
    let resp = client.post(&url).json(params).send().await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        info!(status = status.as_u16(), body = %body, "Adoption rejected by device");
        return Err(ControllerError::DeviceRejected {
            status: status.as_u16(),
            body,
        });
    }

    info!(body = %body, "Adoption accepted by device");
    Ok(AdoptionResult { success: true })
}
