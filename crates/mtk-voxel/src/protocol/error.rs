//! # Protocol Errors
//!
//! Error definitions for packet encoding, decoding, and connection streams.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Packet too short: expected at least {expected} bytes, got {actual}")]
    PacketTooShort { expected: usize, actual: usize },

    #[error("Invalid magic header: expected [0x4D, 0x43], got [0x{0:02X}, 0x{1:02X}]")]
    InvalidMagic(u8, u8),

    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u8),

    #[error("Unknown packet type: 0x{0:02X}")]
    UnknownPacketType(u8),

    #[error("UTF-8 decoding error for field '{field}': {detail}")]
    Utf8Error {
        field: &'static str,
        detail: String,
    },

    #[error("Invalid index format: expected 1 (u8) or 2 (u16), got {0}")]
    InvalidIndexFormat(u8),

    #[error("Payload truncated while parsing {field}")]
    TruncatedPayload { field: &'static str },
}
