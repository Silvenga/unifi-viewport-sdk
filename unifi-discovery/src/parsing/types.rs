/// A TLV type code (1 byte).
pub type TypeCode = u8;

/// A TLV value (raw bytes, owned).
pub type TlvValue = Vec<u8>;

/// A frame command byte.
pub type Command = u8;

/// A protocol version byte.
pub type Version = u8;
