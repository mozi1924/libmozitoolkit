//! # Live Sync Events (RFC v3.1)
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
    SelectionUpdated { min_pos: IVec3, size: IVec3 },

    /// Handshake synchronization metadata.
    Handshake {
        epoch: u64,
        sequence: u64,
        decision: String,
        dimension: String,
    },

    /// A unified, merged world mesh containing the entire active volume has been assembled.
    WorldMeshReady { mesh: MeshData, generation: u64 },

    /// Stream batch progress update.
    StreamProgress {
        stage: String,
        current: usize,
        total: usize,
        message: String,
    },

    /// Stream batch completed.
    StreamFinished { built_sections: usize },

    /// Incremental delta modifications applied.
    DeltaApplied {
        change_count: usize,
        affected_sections: Vec<IVec3>,
    },

    /// Live sync validation completed.
    Verified { is_verified: bool, message: String },

    /// Warning or non-fatal error.
    Warning(String),

    /// Fatal or recoverable error message.
    Error(String),
}
