use crate::parsing::{Command, Version};
use thiserror::Error;

/// Errors returned by the discovery frame parser.
#[derive(Debug, Error)]
pub enum ParsingError {
    /// The input buffer had fewer bytes than required to parse a frame or TLV header.
    #[error("buffer too short: need {needed} bytes, have {available}")]
    BufferTooShort {
        /// Number of bytes required.
        needed: usize,
        /// Number of bytes available in the buffer.
        available: usize,
    },

    /// The frame header carried a protocol version other than `0x01`.
    #[error("unsupported protocol version: expected 0x01, got 0x{0:02X}")]
    UnsupportedVersion(Version),

    /// The frame header carried a command other than `CMD_INFO` (`0x00`).
    #[error("unexpected command: expected 0x00 (CMD_INFO), got 0x{0:02X}")]
    UnexpectedCommand(Command),

    /// The TLV section length declared in the header exceeds the remaining buffer.
    #[error("TLV section length mismatch: header says {declared} bytes, {actual} bytes available")]
    TlvLengthMismatch {
        /// TLV section length declared in the frame header.
        declared: usize,
        /// Actual bytes available after the 4-byte header.
        actual: usize,
    },

    /// A TLV entry's value length exceeds the remaining TLV section.
    #[error("TLV value length exceeds remaining buffer: need {needed}, have {available}")]
    TlvValueTooLong {
        /// Number of bytes the TLV entry claims to need.
        needed: usize,
        /// Bytes remaining in the TLV section.
        available: usize,
    },
}
