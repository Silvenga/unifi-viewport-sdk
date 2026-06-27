//! UniFi Protect device discovery protocol (UDP 10001).
//!
//! Implements the TLV-based query/response protocol used by Ubiquiti devices to announce
//! themselves to controllers on the local network. See the [protocol spec] for details.
//!
//! [protocol spec]: https://github.com/Silvenga/unifi-viewport-sdk/blob/master/spec/discovery.md

mod client;
mod error;
mod message;
mod parsing;
mod responder;
mod types;

pub use client::DiscoveryClient;
pub use error::DiscoveryError;
pub use message::DiscoveryMessage;
pub use parsing::{Command, Frame, ParsingError, TlvValue, TlvValues, TypeCode, Version};
pub use responder::DiscoveryResponder;
pub use types::{DeviceInfo, MacAddress};
