//! # Embedded Minecraft Debug World Fixture
//!
//! Provides the canonical embedded Minecraft debug world snapshot spanning 529 chunk sections
//! and 32,539 block states for instant regression testing, offline benchmarking, and DCC viewport inspection.

use std::sync::OnceLock;
use flate2::read::GzDecoder;
use serde::Deserialize;

use crate::storage::VoxelStorage;
use crate::types::VoxelError;

/// Raw compressed Gzip bytes of the canonical Minecraft debug world snapshot.
pub const DEBUG_WORLD_SNAPSHOT_GZ: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/debug_world_snapshot.json.gz"));

#[derive(Debug, Deserialize)]
struct DebugBlockEntry {
    x: i32,
    y: i32,
    z: i32,
    state: String,
}

#[derive(Debug, Deserialize)]
struct DebugWorldPayload {
    bounds: [i32; 6],
    #[allow(dead_code)]
    total_blocks: usize,
    blocks: Vec<DebugBlockEntry>,
}

static CACHED_DEBUG_STORAGE: OnceLock<VoxelStorage> = OnceLock::new();

impl VoxelStorage {
    /// Deserializes and loads the canonical embedded Minecraft debug world into a new `VoxelStorage` instance.
    ///
    /// This embedded snapshot contains 32,539 block states spanning 529 chunk sections.
    /// The decoded storage is lazily cached internally so subsequent calls clone in sub-millisecond time.
    pub fn create_debug_world() -> Result<Self, VoxelError> {
        if let Some(cached) = CACHED_DEBUG_STORAGE.get() {
            return Ok(cached.clone());
        }

        let mut decoder = GzDecoder::new(DEBUG_WORLD_SNAPSHOT_GZ);
        let payload: DebugWorldPayload = serde_json::from_reader(&mut decoder)
            .map_err(|e| VoxelError::MalformedSnapshot(format!("Failed to parse embedded debug world snapshot: {e}")))?;

        let mut storage = VoxelStorage::new();
        storage.set_bounds(
            payload.bounds[0],
            payload.bounds[1],
            payload.bounds[2],
            payload.bounds[3],
            payload.bounds[4],
            payload.bounds[5],
        );

        for block in payload.blocks {
            storage.set_block(block.x, block.y, block.z, &block.state, None);
        }

        storage.mark_all_sections_dirty();

        let _ = CACHED_DEBUG_STORAGE.set(storage.clone());
        Ok(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_debug_world_loading() {
        let storage = VoxelStorage::create_debug_world().expect("Should load embedded debug world");
        assert_eq!(storage.min_x, 0);
        assert_eq!(storage.min_y, 69);
        assert_eq!(storage.min_z, 0);
        assert_eq!(storage.size_x, 361);
        assert_eq!(storage.size_y, 3);
        assert_eq!(storage.size_z, 363);
        assert_eq!(storage.get_all_non_empty_sections().len(), 529);

        let start = std::time::Instant::now();
        let cloned = VoxelStorage::create_debug_world().expect("Should return cached clone");
        let elapsed = start.elapsed();
        assert_eq!(cloned.get_all_non_empty_sections().len(), 529);
        assert!(elapsed.as_millis() < 100, "Cached clone should take <100ms even in unoptimized debug build");
    }
}
