//! # Strongly Typed Live Sync Binary Packets
//!
//! Representation of all Minecraft (Yefira) <-> DCC Live Sync packet payloads.

use glam::IVec3;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::protocol::constants::StreamStatus;

/// Strongly-typed Live Sync Packet enum.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Packet {
    /// 0x01: Selection Bounding Box Information
    SelectionInfo {
        min_pos: IVec3,
        size: IVec3,
    },

    /// 0x02: Full Selection Snapshot
    FullSnapshot {
        min_pos: IVec3,
        size: IVec3,
        palette: Vec<String>,
        grid_indices: Vec<u16>,
        biome_palette: Option<Vec<String>>,
        biome_indices: Option<Vec<u16>>,
    },

    /// 0x03: Block Delta Updates (incremental modifications)
    DeltaUpdate {
        seq_id: u32,
        min_pos: IVec3,
        changes: Vec<DeltaChange>,
    },

    /// 0x05: Section CRC32 Manifest
    SectionManifest {
        seq_id: u32,
        sections: Vec<ManifestSectionEntry>,
    },

    /// 0x06: Single 16x16x16 Chunk Section Snapshot
    SectionSnapshot {
        sec_coord: IVec3,
        start_pos: IVec3,
        size: IVec3,
        palette: Vec<String>,
        grid_indices: Vec<u16>,
        biome_palette: Option<Vec<String>>,
        biome_indices: Option<Vec<u16>>,
    },

    /// 0x07: Handshake and Metadata
    HandshakeInfo {
        total_sections: u32,
        non_empty_sections: u32,
        total_volume: u32,
        dimension: String,
        flags: u16,
    },

    /// 0x08: Progressive Section Stream Beginning
    StreamBegin {
        stream_id: u32,
        total_sections: u32,
        flags: u16,
    },

    /// 0x09: Progressive Section Stream Ending
    StreamEnd {
        stream_id: u32,
        sent_sections: u32,
        status: StreamStatus,
    },

    /// 0x80: Client requests Full Snapshot
    ReqFullSync,

    /// 0x81: Client requests specific Section Repairs
    ReqSectionSync {
        sections: Vec<IVec3>,
    },

    /// 0x82: Client configuration
    SyncConfig {
        throttle_mode: u8,
        target_fps: u8,
        is_active: bool,
    },
}

/// Single block modification entry in a `DeltaUpdate` packet.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeltaChange {
    pub rel_pos: IVec3,
    pub state: String,
}

/// Single section CRC entry in a `SectionManifest` packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ManifestSectionEntry {
    pub coord: IVec3,
    pub crc32: u32,
}
