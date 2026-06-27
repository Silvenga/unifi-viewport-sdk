use crate::parsing::{Frame, ParsingError, TlvValues};
use crate::types::DeviceInfo;
use crate::Command;

pub const CMD_INFO: Command = 0x00;

/// A discovery protocol message.
///
/// The interpretation layer over [`Frame`]. Produced by parsing raw bytes,
/// consumed by encoding back to raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryMessage {
    /// An empty `CMD_INFO` query (controller → device).
    InfoQuery,
    /// A `CMD_INFO` response with device information (device → controller).
    InfoResponse(DeviceInfo),
}

impl DiscoveryMessage {
    /// Encodes the message into a byte vector.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::InfoQuery => {
                let frame = Frame::new(CMD_INFO, TlvValues::new());
                frame.encode()
            }
            Self::InfoResponse(info) => {
                let frame = Frame::new(CMD_INFO, info.tlvs().clone());
                frame.encode()
            }
        }
    }
}

impl TryFrom<Frame> for DiscoveryMessage {
    type Error = ParsingError;

    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        if frame.command != CMD_INFO {
            return Err(ParsingError::UnexpectedCommand(frame.command));
        }

        if frame.values.is_empty() {
            Ok(Self::InfoQuery)
        } else {
            Ok(Self::InfoResponse(DeviceInfo::from_tlvs(
                frame.values.clone(),
            )))
        }
    }
}

impl TryFrom<&[u8]> for DiscoveryMessage {
    type Error = ParsingError;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        let frame = Frame::parse(buf)?;
        Self::try_from(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::Duration;

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

    #[test]
    fn when_parse_empty_query_then_info_query() {
        let msg = DiscoveryMessage::try_from(&[0x01, 0x00, 0x00, 0x00][..]).unwrap();
        assert_eq!(msg, DiscoveryMessage::InfoQuery);
    }

    #[test]
    fn when_parse_unknown_command_then_error() {
        let buf = [0x01, 0x01, 0x00, 0x00];
        let err = DiscoveryMessage::try_from(&buf[..]).unwrap_err();
        assert!(matches!(err, ParsingError::UnexpectedCommand(1)));
    }

    #[test]
    fn when_parse_response_then_info_response() {
        let msg = DiscoveryMessage::try_from(PRE_ADOPTION_FRAME).unwrap();
        match msg {
            DiscoveryMessage::InfoResponse(info) => {
                assert_eq!(
                    info.get_mac().unwrap().unwrap(),
                    [0xe4, 0x38, 0x83, 0x34, 0x09, 0x1e]
                );
            }
            _ => panic!("expected InfoResponse"),
        }
    }

    #[test]
    fn when_encode_info_query_then_4_bytes() {
        let msg = DiscoveryMessage::InfoQuery;
        assert_eq!(msg.encode(), &[0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn when_encode_info_response_then_roundtrips() {
        let mut info = DeviceInfo::new();
        info.set_mac([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        info.set_ip(
            [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
            Ipv4Addr::new(10, 0, 0, 5),
        );
        info.set_firmware("1.0.0");
        info.set_uptime(Duration::from_secs(300));
        info.set_hostname("Test");
        info.set_is_default(true);

        let msg = DiscoveryMessage::InfoResponse(info);
        let encoded = msg.encode();
        let reparsed = DiscoveryMessage::try_from(encoded.as_slice()).unwrap();

        match reparsed {
            DiscoveryMessage::InfoResponse(decoded) => {
                assert_eq!(
                    decoded.get_mac().unwrap().unwrap(),
                    [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]
                );
                assert_eq!(
                    decoded.get_ip().unwrap().unwrap(),
                    Ipv4Addr::new(10, 0, 0, 5)
                );
                assert_eq!(decoded.get_firmware().unwrap(), "1.0.0");
            }
            _ => panic!("expected InfoResponse"),
        }
    }

    #[test]
    fn when_parse_pre_adoption_then_encode_matches_bytes() {
        let msg = DiscoveryMessage::try_from(PRE_ADOPTION_FRAME).unwrap();
        assert_eq!(msg.encode(), PRE_ADOPTION_FRAME);
    }
}
