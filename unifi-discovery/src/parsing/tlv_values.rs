use crate::parsing::error::ParsingError;
use crate::parsing::{TlvValue, TypeCode};
use indexmap::IndexMap;
use std::str;
use tracing::warn;

pub const TLV_HEADER_LEN: usize = 3;

/// A collection of TLV (Type-Length-Value) entries, keyed by type code.
///
/// Preserves insertion order and supports duplicate type codes (each type code
/// maps to a list of values). Analogous to `http::HeaderMap` — values are raw
/// bytes, with typed accessor methods for common encodings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TlvValues {
    entries: IndexMap<TypeCode, Vec<TlvValue>>,
}

impl TlvValues {
    /// Creates an empty `TlvValues`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the first value for the given type code, or `None` if absent.
    ///
    /// Emits a warning if multiple values exist for the type code.
    pub fn get_first(&self, type_code: TypeCode) -> Option<&[u8]> {
        let values = self.entries.get(&type_code)?;
        if values.len() > 1 {
            warn!(
                type_code = format!("0x{type_code:02X}"),
                count = values.len(),
                "Multiple TLV values for type code, using first"
            );
        }
        values.first().map(|v| v.as_slice())
    }

    /// Returns the first value for the given type code as a UTF-8 string slice.
    pub fn get_first_str(&self, type_code: TypeCode) -> Option<&str> {
        self.get_first(type_code)
            .map(|v| str::from_utf8(v).unwrap_or(""))
    }

    /// Returns the first value as a big-endian `u32`.
    pub fn get_first_u32_be(&self, type_code: TypeCode) -> Option<u32> {
        let value = self.get_first(type_code)?;
        if value.len() == 4 {
            Some(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
        } else {
            None
        }
    }

    /// Returns the first value as a big-endian `u16`.
    pub fn get_first_u16_be(&self, type_code: TypeCode) -> Option<u16> {
        let value = self.get_first(type_code)?;
        if value.len() == 2 {
            Some(u16::from_be_bytes([value[0], value[1]]))
        } else {
            None
        }
    }

    /// Returns the first value as a `u8`.
    pub fn get_first_u8(&self, type_code: TypeCode) -> Option<u8> {
        let value = self.get_first(type_code)?;
        if value.len() == 1 {
            Some(value[0])
        } else {
            None
        }
    }

    /// Returns all values for the given type code.
    pub fn get_all(&self, type_code: TypeCode) -> Option<&Vec<TlvValue>> {
        self.entries.get(&type_code)
    }

    /// Sets the value for a type code, replacing any existing values.
    pub fn set(&mut self, type_code: TypeCode, value: TlvValue) {
        self.entries.insert(type_code, vec![value]);
    }

    /// Appends a value for a type code, preserving existing values.
    pub fn append(&mut self, type_code: TypeCode, value: TlvValue) {
        self.entries.entry(type_code).or_default().push(value);
    }

    /// Removes all values for a type code.
    pub fn remove(&mut self, type_code: TypeCode) -> Option<Vec<TlvValue>> {
        self.entries.shift_remove(&type_code)
    }

    /// Returns the number of distinct type codes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if there are no TLV entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over `(type_code, values)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeCode, &Vec<TlvValue>)> {
        self.entries.iter()
    }

    /// Returns the total encoded size of all TLV entries (headers + values).
    pub fn encoded_size(&self) -> usize {
        self.entries
            .iter()
            .flat_map(|(_, values)| values.iter())
            .map(|v| 3 + v.len())
            .sum()
    }

    /// Writes all TLV entries into `buf` in insertion order.
    ///
    /// If any value exceeds 255 bytes (the TLV length field is a single byte),
    /// a warning is emitted and the value is truncated to 255 bytes.
    pub fn write_to(&self, buf: &mut Vec<u8>) {
        for (type_code, values) in self.entries.iter() {
            for value in values {
                if value.len() > u8::MAX as usize {
                    warn!(
                        type_code = format!("0x{type_code:02X}"),
                        len = value.len(),
                        "TLV value exceeds 255 bytes, truncating to fit uint8 length field"
                    );
                }
                let len = value.len().min(u8::MAX as usize);
                buf.push(*type_code);
                buf.push(0x00);
                buf.push(len as u8);
                buf.extend_from_slice(&value[..len]);
            }
        }
    }

    /// Parses TLV entries from `buf` into an `TlvValues`, consuming the slice.
    pub fn parse(buf: &mut &[u8]) -> Result<Self, ParsingError> {
        let mut entries: IndexMap<TypeCode, Vec<TlvValue>> = IndexMap::new();

        while !buf.is_empty() {
            if buf.len() < TLV_HEADER_LEN {
                return Err(ParsingError::BufferTooShort {
                    needed: TLV_HEADER_LEN,
                    available: buf.len(),
                });
            }

            let type_code = buf[0];
            let reserved = buf[1];
            let length = buf[2] as usize;

            if reserved != 0x00 {
                warn!(
                    type_code = format!("0x{type_code:02X}"),
                    reserved = format!("0x{reserved:02X}"),
                    "Non-zero reserved byte in TLV entry"
                );
            }

            *buf = &buf[TLV_HEADER_LEN..];

            if buf.len() < length {
                return Err(ParsingError::TlvValueTooLong {
                    needed: length,
                    available: buf.len(),
                });
            }

            let value = buf[..length].to_vec();
            *buf = &buf[length..];

            entries.entry(type_code).or_default().push(value);
        }

        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_set_then_get_first_returns_value() {
        let mut tv = TlvValues::new();
        tv.set(0x01, vec![0xAA, 0xBB]);
        assert_eq!(tv.get_first(0x01), Some(&[0xAA, 0xBB][..]));
    }

    #[test]
    fn when_append_then_get_all_returns_all_values() {
        let mut tv = TlvValues::new();
        tv.append(0x01, vec![0xAA]);
        tv.append(0x01, vec![0xBB]);
        assert_eq!(tv.get_all(0x01).unwrap().len(), 2);
    }

    #[test]
    fn when_set_replaces_existing() {
        let mut tv = TlvValues::new();
        tv.append(0x01, vec![0xAA]);
        tv.append(0x01, vec![0xBB]);
        tv.set(0x01, vec![0xCC]);
        assert_eq!(tv.get_first(0x01), Some(&[0xCC][..]));
        assert_eq!(tv.get_all(0x01).unwrap().len(), 1);
    }

    #[test]
    fn when_remove_then_entry_gone() {
        let mut tv = TlvValues::new();
        tv.set(0x01, vec![0xAA]);
        assert!(tv.remove(0x01).is_some());
        assert!(tv.get_first(0x01).is_none());
    }

    #[test]
    fn when_get_first_str_then_returns_utf8() {
        let mut tv = TlvValues::new();
        tv.set(0x03, b"hello".to_vec());
        assert_eq!(tv.get_first_str(0x03), Some("hello"));
    }

    #[test]
    fn when_get_first_u32_be_then_returns_big_endian() {
        let mut tv = TlvValues::new();
        tv.set(0x0A, 300u32.to_be_bytes().to_vec());
        assert_eq!(tv.get_first_u32_be(0x0A), Some(300));
    }

    #[test]
    fn when_get_first_u16_be_then_returns_big_endian() {
        let mut tv = TlvValues::new();
        tv.set(0x10, 0x80E9u16.to_be_bytes().to_vec());
        assert_eq!(tv.get_first_u16_be(0x10), Some(0x80E9));
    }

    #[test]
    fn when_get_first_u8_then_returns_value() {
        let mut tv = TlvValues::new();
        tv.set(0x2C, vec![0x03]);
        assert_eq!(tv.get_first_u8(0x2C), Some(0x03));
    }

    #[test]
    fn when_write_to_then_preserves_insertion_order() {
        let mut tv = TlvValues::new();
        tv.set(0x03, b"fw".to_vec());
        tv.set(0x01, vec![0xAA, 0xBB]);

        let mut buf = Vec::new();
        tv.write_to(&mut buf);

        assert_eq!(buf[0], 0x03);
        assert_eq!(buf[5], 0x01);
    }

    #[test]
    fn when_parse_then_roundtrips_through_write_to() {
        let mut original = TlvValues::new();
        original.set(0x01, vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
        original.set(0x03, b"firmware".to_vec());
        original.set(0x0A, 42u32.to_be_bytes().to_vec());

        let mut buf = Vec::new();
        original.write_to(&mut buf);

        let mut input = buf.as_slice();
        let parsed = TlvValues::parse(&mut input).unwrap();
        assert!(input.is_empty());
        assert_eq!(parsed, original);
    }

    #[test]
    fn when_parse_with_duplicates_then_preserves_all() {
        let raw: &[u8] = &[0x01, 0x00, 0x02, 0xAA, 0xBB, 0x01, 0x00, 0x02, 0xCC, 0xDD];
        let mut input = raw;
        let parsed = TlvValues::parse(&mut input).unwrap();
        assert_eq!(parsed.get_all(0x01).unwrap().len(), 2);
    }

    #[test]
    fn when_encoded_size_then_sums_headers_and_values() {
        let mut tv = TlvValues::new();
        tv.set(0x01, vec![0xAA, 0xBB]);
        tv.set(0x03, b"fw".to_vec());
        assert_eq!(tv.encoded_size(), (3 + 2) + (3 + 2));
    }
}
