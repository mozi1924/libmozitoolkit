#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Protocol magic header bytes: 'M' (0x4D), 'C' (0x43).
pub const PROTOCOL_MAGIC: [u8; 2] = [0x4D, 0x43];

/// Protocol version number (Version 1).
pub const PROTOCOL_VERSION: u8 = 0x01;

/// Packet Type IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum PacketType {
    // S -> C Packets (Server to Client)
    SelectionInfo = 0x01,
    FullSnapshot = 0x02,
    DeltaUpdate = 0x03,
    SectionManifest = 0x05,
    SectionSnapshot = 0x06,
    HandshakeInfo = 0x07,
    StreamBegin = 0x08,
    StreamEnd = 0x09,

    // C -> S Packets (Client to Server)
    ReqFullSync = 0x80,
    ReqSectionSync = 0x81,
    SyncConfig = 0x82,
}

impl PacketType {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(PacketType::SelectionInfo),
            0x02 => Some(PacketType::FullSnapshot),
            0x03 => Some(PacketType::DeltaUpdate),
            0x05 => Some(PacketType::SectionManifest),
            0x06 => Some(PacketType::SectionSnapshot),
            0x07 => Some(PacketType::HandshakeInfo),
            0x08 => Some(PacketType::StreamBegin),
            0x09 => Some(PacketType::StreamEnd),
            0x80 => Some(PacketType::ReqFullSync),
            0x81 => Some(PacketType::ReqSectionSync),
            0x82 => Some(PacketType::SyncConfig),
            _ => None,
        }
    }
}

// Fixed Header and Payload Sizes (in bytes)
pub const HEADER_SIZE: usize = 4; // Magic(2) + Version(1) + Type(1)
pub const SELECTION_INFO_SIZE: usize = 24; // 6 * i32
pub const DELTA_HEADER_SIZE: usize = 18; // seq_id(u32) + min_x..z(3*i32) + count(u16)
pub const DELTA_CHANGE_PREFIX_SIZE: usize = 8; // rel_x..z(3*u16) + str_len(u16)
pub const MANIFEST_HEADER_SIZE: usize = 8; // seq_id(u32) + count(u32)
pub const MANIFEST_ENTRY_SIZE: usize = 16; // sec_x..z(3*i32) + crc32(u32)
pub const SECTION_SNAPSHOT_HEADER_SIZE: usize = 38; // sec_x..z(3*i32) + start_x..z(3*i32) + size_x..z(3*i32) + palette_count(u16)
pub const HANDSHAKE_INFO_HEADER_SIZE: usize = 14; // total_sec(u32) + non_empty_sec(u32) + volume(u32) + dim_len(u16)
pub const STREAM_BEGIN_SIZE: usize = 10; // stream_id(u32) + total_sec(u32) + flags(u16)
pub const STREAM_END_SIZE: usize = 10; // stream_id(u32) + sent_sec(u32) + status(u16)

/// Stream status codes reported in `StreamEnd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u16)]
pub enum StreamStatus {
    Success = 0,
    Cancelled = 1,
    Error = 2,
}

impl StreamStatus {
    pub fn from_u16(val: u16) -> Self {
        match val {
            0 => StreamStatus::Success,
            1 => StreamStatus::Cancelled,
            _ => StreamStatus::Error,
        }
    }
}
