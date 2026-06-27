use crate::parsing::{ParsingError, TlvValues, TypeCode};
use std::net::Ipv4Addr;
use std::time::Duration;

pub const TYPE_MAC: TypeCode = 0x01;
pub const TYPE_MAC_IP: TypeCode = 0x02;
pub const TYPE_FIRMWARE: TypeCode = 0x03;
pub const TYPE_UPTIME: TypeCode = 0x0A;
pub const TYPE_HOSTNAME: TypeCode = 0x0B;
pub const TYPE_PLATFORM: TypeCode = 0x0C;
pub const TYPE_IS_DEFAULT: TypeCode = 0x17;
pub const TYPE_GUID: TypeCode = 0x20;
pub const TYPE_DEVICE_ID: TypeCode = 0x2B;
pub const TYPE_NVR_HARDWARE_ID: TypeCode = 0x26;

/// A 6-byte MAC address.
pub type MacAddress = [u8; 6];

/// Device information decoded from a discovery response.
///
/// A thin wrapper over a [`TlvValues`] bag with typed accessors for known
/// TLV fields. Unknown TLV types are preserved in the bag and accessible
/// via [`tlvs()`](Self::tlvs).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceInfo {
    tlvs: TlvValues,
}

impl DeviceInfo {
    /// Creates an empty `DeviceInfo`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `DeviceInfo` from a `TlvValues` bag.
    pub fn from_tlvs(tlvs: TlvValues) -> Self {
        Self { tlvs }
    }

    /// Returns the underlying TLV values.
    pub fn tlvs(&self) -> &TlvValues {
        &self.tlvs
    }

    /// Returns a mutable reference to the underlying TLV values.
    pub fn tlvs_mut(&mut self) -> &mut TlvValues {
        &mut self.tlvs
    }

    /// Returns the device MAC address (TLV type `0x01`).
    pub fn get_mac(&self) -> Result<Option<MacAddress>, ParsingError> {
        let value = match self.tlvs.get_first(TYPE_MAC) {
            Some(v) => v,
            None => return Ok(None),
        };
        if value.len() == 6 {
            let mut mac = [0u8; 6];
            mac.copy_from_slice(value);
            Ok(Some(mac))
        } else {
            Err(ParsingError::BufferTooShort {
                needed: 6,
                available: value.len(),
            })
        }
    }

    /// Sets the device MAC address (TLV type `0x01`).
    pub fn set_mac(&mut self, mac: MacAddress) {
        self.tlvs.set(TYPE_MAC, mac.to_vec());
    }

    /// Returns the device IPv4 address (TLV type `0x02`).
    pub fn get_ip(&self) -> Result<Option<Ipv4Addr>, ParsingError> {
        let value = match self.tlvs.get_first(TYPE_MAC_IP) {
            Some(v) => v,
            None => return Ok(None),
        };
        if value.len() == 10 {
            Ok(Some(Ipv4Addr::new(value[6], value[7], value[8], value[9])))
        } else {
            Err(ParsingError::BufferTooShort {
                needed: 10,
                available: value.len(),
            })
        }
    }

    /// Sets the MAC + IP address (TLV type `0x02`).
    pub fn set_ip(&mut self, mac: MacAddress, ip: Ipv4Addr) {
        let mut mac_ip = mac.to_vec();
        mac_ip.extend_from_slice(&ip.octets());
        self.tlvs.set(TYPE_MAC_IP, mac_ip);
    }

    /// Returns the firmware version string (TLV type `0x03`).
    pub fn get_firmware(&self) -> Option<String> {
        self.tlvs
            .get_first(TYPE_FIRMWARE)
            .map(|v| String::from_utf8_lossy(v).into_owned())
    }

    /// Sets the firmware version string (TLV type `0x03`).
    pub fn set_firmware(&mut self, firmware: &str) {
        self.tlvs.set(TYPE_FIRMWARE, firmware.as_bytes().to_vec());
    }

    /// Returns the device uptime (TLV type `0x0A`).
    pub fn get_uptime(&self) -> Result<Option<Duration>, ParsingError> {
        Ok(self
            .tlvs
            .get_first_u32_be(TYPE_UPTIME)
            .map(|secs| Duration::from_secs(secs as u64)))
    }

    /// Sets the device uptime (TLV type `0x0A`).
    pub fn set_uptime(&mut self, uptime: Duration) {
        self.tlvs.set(
            TYPE_UPTIME,
            (uptime.as_secs() as u32).to_be_bytes().to_vec(),
        );
    }

    /// Returns the hostname (TLV type `0x0B`).
    pub fn get_hostname(&self) -> Option<String> {
        self.tlvs
            .get_first(TYPE_HOSTNAME)
            .map(|v| String::from_utf8_lossy(v).into_owned())
    }

    /// Sets the hostname (TLV type `0x0B`).
    pub fn set_hostname(&mut self, hostname: &str) {
        self.tlvs.set(TYPE_HOSTNAME, hostname.as_bytes().to_vec());
    }

    /// Returns the platform / short model (TLV type `0x0C`).
    pub fn get_platform(&self) -> Option<String> {
        self.tlvs
            .get_first(TYPE_PLATFORM)
            .map(|v| String::from_utf8_lossy(v).into_owned())
    }

    /// Sets the platform / short model (TLV type `0x0C`).
    pub fn set_platform(&mut self, platform: &str) {
        self.tlvs.set(TYPE_PLATFORM, platform.as_bytes().to_vec());
    }

    /// Returns whether the device is in factory-default state (TLV type `0x17`).
    pub fn get_is_default(&self) -> Result<Option<bool>, ParsingError> {
        let value = match self.tlvs.get_first(TYPE_IS_DEFAULT) {
            Some(v) => v,
            None => return Ok(None),
        };
        if value.len() == 4 {
            let val = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
            Ok(Some(val != 0))
        } else {
            Err(ParsingError::BufferTooShort {
                needed: 4,
                available: value.len(),
            })
        }
    }

    /// Sets whether the device is in factory-default state (TLV type `0x17`).
    pub fn set_is_default(&mut self, is_default: bool) {
        let val: u32 = if is_default { 1 } else { 0 };
        self.tlvs.set(TYPE_IS_DEFAULT, val.to_be_bytes().to_vec());
    }

    /// Returns the device GUID as a UUID string (TLV type `0x20`).
    pub fn get_guid(&self) -> Option<String> {
        self.tlvs
            .get_first(TYPE_GUID)
            .map(|v| String::from_utf8_lossy(v).into_owned())
    }

    /// Sets the device GUID as a UUID string (TLV type `0x20`).
    pub fn set_guid(&mut self, guid: &str) {
        self.tlvs.set(TYPE_GUID, guid.as_bytes().to_vec());
    }

    /// Returns the binary device ID (TLV type `0x2B`).
    pub fn get_device_id(&self) -> Result<Option<[u8; 16]>, ParsingError> {
        let value = match self.tlvs.get_first(TYPE_DEVICE_ID) {
            Some(v) => v,
            None => return Ok(None),
        };
        if value.len() == 16 {
            let mut id = [0u8; 16];
            id.copy_from_slice(value);
            Ok(Some(id))
        } else {
            Err(ParsingError::BufferTooShort {
                needed: 16,
                available: value.len(),
            })
        }
    }

    /// Sets the binary device ID (TLV type `0x2B`).
    pub fn set_device_id(&mut self, device_id: [u8; 16]) {
        self.tlvs.set(TYPE_DEVICE_ID, device_id.to_vec());
    }

    /// Returns the NVR hardware ID, present only when adopted (TLV type `0x26`).
    pub fn get_nvr_hardware_id(&self) -> Result<Option<[u8; 16]>, ParsingError> {
        let value = match self.tlvs.get_first(TYPE_NVR_HARDWARE_ID) {
            Some(v) => v,
            None => return Ok(None),
        };
        if value.len() == 16 {
            let mut id = [0u8; 16];
            id.copy_from_slice(value);
            Ok(Some(id))
        } else {
            Err(ParsingError::BufferTooShort {
                needed: 16,
                available: value.len(),
            })
        }
    }

    /// Sets the NVR hardware ID (TLV type `0x26`).
    pub fn set_nvr_hardware_id(&mut self, nvr_hardware_id: [u8; 16]) {
        self.tlvs
            .set(TYPE_NVR_HARDWARE_ID, nvr_hardware_id.to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CMD_INFO;
    use crate::parsing::{Frame, TlvValues};

    const PRE_ADOPTION_FRAME: &[u8] = &[
        0x01, 0x00, 0x00, 0xb7, 0x01, 0x00, 0x06, 0xe4, 0x38, 0x83, 0x34, 0x09, 0x1e, 0x02, 0x00,
        0x0a, 0xe4, 0x38, 0x83, 0x34, 0x09, 0x1e, 0xc0, 0xa8, 0x00, 0xc9, 0x03, 0x00, 0x2a, 0x55,
        0x50, 0x56, 0x2e, 0x71, 0x63, 0x73, 0x36, 0x30, 0x35, 0x2e, 0x76, 0x31, 0x2e, 0x34, 0x2e,
        0x33, 0x33, 0x2e, 0x30, 0x2e, 0x34, 0x36, 0x39, 0x38, 0x64, 0x61, 0x66, 0x32, 0x36, 0x2e,
        0x32, 0x36, 0x30, 0x34, 0x31, 0x36, 0x2e, 0x31, 0x31, 0x31, 0x34, 0x0a, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x4e, 0x0b, 0x00, 0x0b, 0x55, 0x50, 0x20, 0x56, 0x69, 0x65, 0x77, 0x70, 0x6f,
        0x72, 0x74, 0x0c, 0x00, 0x0b, 0x55, 0x50, 0x20, 0x56, 0x69, 0x65, 0x77, 0x70, 0x6f, 0x72,
        0x74, 0x17, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x2c, 0x00, 0x01, 0x03, 0x10, 0x00, 0x02,
        0x80, 0xe9, 0x0f, 0x00, 0x04, 0x00, 0x01, 0x1f, 0x90, 0x20, 0x00, 0x24, 0x37, 0x66, 0x39,
        0x63, 0x39, 0x30, 0x61, 0x32, 0x2d, 0x38, 0x31, 0x35, 0x32, 0x2d, 0x35, 0x64, 0x36, 0x33,
        0x2d, 0x32, 0x31, 0x34, 0x62, 0x2d, 0x64, 0x39, 0x36, 0x64, 0x36, 0x64, 0x38, 0x39, 0x34,
        0x62, 0x31, 0x66, 0x2b, 0x00, 0x10, 0x13, 0x85, 0xfe, 0x74, 0x06, 0xad, 0x49, 0x6f, 0x93,
        0x3e, 0xc1, 0x78, 0x5e, 0x3d, 0x79, 0x47,
    ];

    const POST_ADOPTION_FRAME: &[u8] = &[
        0x01, 0x00, 0x00, 0xca, 0x01, 0x00, 0x06, 0xe4, 0x38, 0x83, 0x34, 0x09, 0x1e, 0x02, 0x00,
        0x0a, 0xe4, 0x38, 0x83, 0x34, 0x09, 0x1e, 0xc0, 0xa8, 0x00, 0xc9, 0x03, 0x00, 0x2a, 0x55,
        0x50, 0x56, 0x2e, 0x71, 0x63, 0x73, 0x36, 0x30, 0x35, 0x2e, 0x76, 0x31, 0x2e, 0x34, 0x2e,
        0x33, 0x33, 0x2e, 0x30, 0x2e, 0x34, 0x36, 0x39, 0x38, 0x64, 0x61, 0x66, 0x32, 0x36, 0x2e,
        0x32, 0x36, 0x30, 0x34, 0x31, 0x36, 0x2e, 0x31, 0x31, 0x31, 0x34, 0x0a, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x59, 0x0b, 0x00, 0x0b, 0x55, 0x50, 0x20, 0x56, 0x69, 0x65, 0x77, 0x70, 0x6f,
        0x72, 0x74, 0x0c, 0x00, 0x0b, 0x55, 0x50, 0x20, 0x56, 0x69, 0x65, 0x77, 0x70, 0x6f, 0x72,
        0x74, 0x17, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x01, 0x03, 0x10, 0x00, 0x02,
        0x80, 0xe9, 0x0f, 0x00, 0x04, 0x00, 0x01, 0x1f, 0x90, 0x20, 0x00, 0x24, 0x37, 0x66, 0x39,
        0x63, 0x39, 0x30, 0x61, 0x32, 0x2d, 0x38, 0x31, 0x35, 0x32, 0x2d, 0x35, 0x64, 0x36, 0x33,
        0x2d, 0x32, 0x31, 0x34, 0x62, 0x2d, 0x64, 0x39, 0x36, 0x64, 0x36, 0x64, 0x38, 0x39, 0x34,
        0x62, 0x31, 0x66, 0x2b, 0x00, 0x10, 0x13, 0x85, 0xfe, 0x74, 0x06, 0xad, 0x49, 0x6f, 0x93,
        0x3e, 0xc1, 0x78, 0x5e, 0x3d, 0x79, 0x47, 0x26, 0x00, 0x10, 0x53, 0x54, 0x0e, 0xa4, 0xb5,
        0x20, 0x51, 0x2c, 0xaf, 0x90, 0xef, 0x08, 0xf1, 0x0e, 0xb2, 0xaa,
    ];

    fn frame_to_device_info(raw: &[u8]) -> DeviceInfo {
        let frame = Frame::parse(raw).unwrap();
        DeviceInfo::from_tlvs(frame.values.clone())
    }

    #[test]
    fn when_decode_pre_adoption_then_fields_match_spec() {
        let info = frame_to_device_info(PRE_ADOPTION_FRAME);

        assert_eq!(
            info.get_mac().unwrap().unwrap(),
            [0xe4, 0x38, 0x83, 0x34, 0x09, 0x1e]
        );
        assert_eq!(
            info.get_ip().unwrap().unwrap(),
            Ipv4Addr::new(192, 168, 0, 201)
        );
        assert_eq!(
            info.get_firmware().unwrap(),
            "UPV.qcs605.v1.4.33.0.4698daf26.260416.1114"
        );
        assert_eq!(info.get_uptime().unwrap().unwrap(), Duration::from_secs(78));
        assert_eq!(info.get_hostname().unwrap(), "UP Viewport");
        assert_eq!(info.get_platform().unwrap(), "UP Viewport");
        assert!(info.get_is_default().unwrap().unwrap());
        assert_eq!(
            info.get_guid().unwrap(),
            "7f9c90a2-8152-5d63-214b-d96d6d894b1f"
        );
        assert_eq!(
            info.get_device_id().unwrap().unwrap(),
            [
                0x13, 0x85, 0xfe, 0x74, 0x06, 0xad, 0x49, 0x6f, 0x93, 0x3e, 0xc1, 0x78, 0x5e, 0x3d,
                0x79, 0x47
            ]
        );
        assert!(info.get_nvr_hardware_id().unwrap().is_none());
    }

    #[test]
    fn when_decode_post_adoption_then_nvr_hardware_id_present_and_is_default_false() {
        let info = frame_to_device_info(POST_ADOPTION_FRAME);

        assert!(!info.get_is_default().unwrap().unwrap());
        assert_eq!(info.get_uptime().unwrap().unwrap(), Duration::from_secs(89));
        assert_eq!(
            info.get_nvr_hardware_id().unwrap().unwrap(),
            [
                0x53, 0x54, 0x0e, 0xa4, 0xb5, 0x20, 0x51, 0x2c, 0xaf, 0x90, 0xef, 0x08, 0xf1, 0x0e,
                0xb2, 0xaa
            ]
        );
    }

    #[test]
    fn when_decode_empty_tlvs_then_all_getters_return_none() {
        let info = DeviceInfo::new();
        assert!(info.get_mac().unwrap().is_none());
        assert!(info.get_ip().unwrap().is_none());
        assert!(info.get_is_default().unwrap().is_none());
    }

    #[test]
    fn when_decode_unknown_tlv_then_preserved_in_bag() {
        let mut tlvs = TlvValues::new();
        tlvs.set(0xFF, vec![0xAB, 0xCD]);
        let info = DeviceInfo::from_tlvs(tlvs);

        assert!(info.tlvs().get_first(0xFF).is_some());
        assert!(info.get_mac().unwrap().is_none());
    }

    #[test]
    fn when_set_fields_then_encode_roundtrips() {
        let mut info = DeviceInfo::new();
        info.set_mac([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        info.set_ip(
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            Ipv4Addr::new(10, 0, 0, 5),
        );
        info.set_firmware("1.0.0");
        info.set_uptime(Duration::from_secs(300));
        info.set_hostname("Test");
        info.set_platform("Test");
        info.set_is_default(true);
        info.set_guid("550e8400-e29b-41d4-a716-446655440000");

        let frame = Frame::new(CMD_INFO, info.tlvs().clone());
        let encoded = frame.encode();
        let reparsed = Frame::parse(&encoded).unwrap();
        let decoded = DeviceInfo::from_tlvs(reparsed.values.clone());

        assert_eq!(
            decoded.get_mac().unwrap().unwrap(),
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
        );
        assert_eq!(
            decoded.get_ip().unwrap().unwrap(),
            Ipv4Addr::new(10, 0, 0, 5)
        );
        assert_eq!(decoded.get_firmware().unwrap(), "1.0.0");
        assert_eq!(
            decoded.get_uptime().unwrap().unwrap(),
            Duration::from_secs(300)
        );
        assert_eq!(decoded.get_hostname().unwrap(), "Test");
        assert_eq!(decoded.get_platform().unwrap(), "Test");
        assert!(decoded.get_is_default().unwrap().unwrap());
        assert_eq!(
            decoded.get_guid().unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }
}
