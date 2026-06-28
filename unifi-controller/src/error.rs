use thiserror::Error;

/// Errors from the controller-side adoption flow.
#[derive(Debug, Error)]
pub enum ControllerError {
    /// The HTTP request failed (network error, TLS error, etc.).
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The device returned a non-success HTTP status code.
    #[error("device returned HTTP {status}: {body}")]
    DeviceRejected {
        /// The HTTP status code returned by the device.
        status: u16,
        /// The response body from the device.
        body: String,
    },
}
