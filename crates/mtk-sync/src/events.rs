//! # Live Sync Events
//!
//! Event types dispatched from the Rust background sync thread to host applications.

use glam::IVec3;
use mtk_core::mesh::MeshData;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// High-level event produced by `LiveSyncSession`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SyncEvent {
    /// Connection lifecycle status has changed.
    StatusChange(String),

    /// Minecraft selection bounding box updated.
    SelectionUpdated {
        min_pos: IVec3,
        size: IVec3,
    },

    /// Handshake synchronization metadata.
    Handshake {
        total_sections: u32,
        non_empty_sections: u32,
        total_volume: u32,
        dimension: String,
        flags: u16,
    },

    /// A 16x16x16 chunk section mesh has been assembled in background and is ready for DCC ingestion.
    SectionMeshReady {
        coord: IVec3,
        mesh: MeshData,
    },

    /// A unified, merged world mesh containing the entire active volume has been assembled.
    WorldMeshReady {
        mesh: MeshData,
    },

    /// Stream batch progress update.
    StreamProgress {
        current: usize,
        total: usize,
        message: String,
    },

    /// Stream batch completed.
    StreamFinished {
        stream_id: u32,
        built_sections: usize,
    },

    /// Incremental delta modifications applied.
    DeltaApplied {
        change_count: usize,
        affected_sections: Vec<IVec3>,
    },

    /// Live sync validation completed.
    Verified {
        is_verified: bool,
        message: String,
    },

    /// Warning or non-fatal error.
    Warning(String),

    /// Fatal or recoverable error message.
    Error(String),
}
