use serde::{Deserialize, Serialize};

/// The persisted state of a Viewport device across restarts.
///
/// In the factory-default state, no `DeviceState` exists (or `is_adopted` is
/// `false`). Once adopted by a controller, the device stores the controller's
/// identity and its own client certificate so it can reconnect after reboots
/// or controller IP changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceState {
    /// Whether the device has been adopted by a controller.
    pub is_adopted: bool,

    /// The device user's password. Defaults to `ui` on factory devices.
    /// The controller can change this via `changeUserPassword`.
    pub password: String,

    /// The controller's console ID, used to identify the same controller
    /// across IP changes. Set on first adoption.
    pub controller_id: Option<String>,

    /// The controller's console display name.
    pub controller_name: Option<String>,

    /// The DER-encoded client certificate used for the UCP4 WebSocket
    /// connection to the controller. Generated on adoption; the controller
    /// pins its fingerprint.
    pub client_cert_der: Option<Vec<u8>>,

    /// The DER-encoded private key for the client certificate.
    pub client_key_der: Option<Vec<u8>>,

    /// The device's unique 16-byte identifier (TLV type 0x2B).
    /// Generated on first boot and persisted across restarts.
    pub guid: [u8; 16],

    /// The controller's hardware ID (TLV type 0x26). Binary 16-byte value
    /// derived from the controller's console ID. Included in post-adoption
    /// discovery responses.
    pub controller_id_binary: Option<[u8; 16]>,
}

impl DeviceState {
    /// Creates a factory-default device state with a random device ID.
    pub fn factory_default() -> Self {
        Self {
            is_adopted: false,
            password: "ubnt".to_owned(),
            controller_id: None,
            controller_name: None,
            client_cert_der: None,
            client_key_der: None,
            guid: generate_guid(),
            controller_id_binary: None,
        }
    }

    /// Returns whether the device should appear as adoptable in discovery
    /// responses (factory default, not yet adopted).
    pub fn is_adoptable(&self) -> bool {
        !self.is_adopted
    }
}

fn generate_guid() -> [u8; 16] {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = now.as_nanos();
    let mut id = [0u8; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = ((nanos >> (i * 8)) & 0xFF) as u8 ^ (i as u8).wrapping_mul(0x37);
    }
    id
}
