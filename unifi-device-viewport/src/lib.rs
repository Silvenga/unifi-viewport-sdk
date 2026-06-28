//! Device-side implementation of the Ubiquiti Protect Viewport.
//!
//! The [`ViewPortDevice`] struct is the primary entry point for consumers.
//! It orchestrates both the UDP discovery responder (port 10001) and the
//! TLS adoption server (port 8080), managing device state transitions
//! from factory-default to adopted.
//!
//! State is persisted via a consumer-provided [`DeviceStorage`] impl,
//! allowing the device to restore its adopted state across restarts.

mod cert;
mod device;
mod error;
mod routes;
mod server;
mod state;
mod storage;

pub use device::{ViewPortDevice, ViewPortDeviceBuilder};
pub use error::DeviceError;
pub use routes::{AdoptionCallback, AdoptionRequest, InfoRequest, InfoResponse, SettingsRequest};
pub use server::DeviceServer;
pub use state::DeviceState;
pub use storage::{DeviceStorage, InMemoryStorage, SharedStorage};
