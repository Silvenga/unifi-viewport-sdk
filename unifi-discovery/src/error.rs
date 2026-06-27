use crate::parsing::ParsingError;
use std::io;
use thiserror::Error;

/// Errors returned by the discovery protocol.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// A parsing error occurred while decoding a discovery frame.
    #[error("parsing error: {0}")]
    Parsing(#[from] ParsingError),

    /// An I/O error occurred while sending or receiving on the UDP socket.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}
