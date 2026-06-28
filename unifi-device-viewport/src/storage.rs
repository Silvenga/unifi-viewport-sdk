use crate::state::DeviceState;
use std::sync::Arc;
use std::sync::Mutex;

/// Persistent storage for device state across restarts.
///
/// The device needs to persist:
/// - The device password (the controller can change it via `changeUserPassword`).
/// - The controller's console ID (to identify the same controller across IP changes).
/// - The device's client certificate (the controller pins its fingerprint).
///
/// Implementors may back this with a file, database, or any persistent store.
/// The [`InMemoryStorage`] implementation is available for testing.
pub trait DeviceStorage: Send + Sync {
    /// Loads the persisted device state, or `None` if no state exists (factory default).
    fn load(&self) -> Result<Option<DeviceState>, String>;

    /// Persists the device state, replacing any previous state.
    fn save(&self, state: &DeviceState) -> Result<(), String>;
}

/// In-memory storage for testing and ephemeral use.
#[derive(Debug, Default)]
pub struct InMemoryStorage {
    state: Mutex<Option<DeviceState>>,
}

impl InMemoryStorage {
    /// Creates a new empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl DeviceStorage for InMemoryStorage {
    fn load(&self) -> Result<Option<DeviceState>, String> {
        Ok(self.state.lock().unwrap().clone())
    }

    fn save(&self, state: &DeviceState) -> Result<(), String> {
        *self.state.lock().unwrap() = Some(state.clone());
        Ok(())
    }
}

/// Wraps any `DeviceStorage` in an `Arc` for shared access.
pub type SharedStorage = Arc<dyn DeviceStorage>;
