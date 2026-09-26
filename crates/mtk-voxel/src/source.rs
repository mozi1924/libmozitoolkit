//! # Voxel Source and Access Abstractions
//!
//! Defines host-agnostic traits for reading, mutating, and streaming 3D voxel data.
//! This cleanly decouples storage and meshing algorithms in `mtk-voxel` from upstream
//! input providers (e.g. Anvil/MCA saves, Bedrock LevelDB, Live Sync streams, procedural
//! generators, or mesh reverse-engineering guessers).

use glam::IVec3;
use crate::storage::SectionStorage;
use crate::types::VoxelError;

/// Read-only spatial access to a 3D block/voxel world.
pub trait VoxelReader {
    /// Returns the blockstate string at world block coordinates `(x, y, z)`.
    /// If air or outside defined bounds, returns `"minecraft:air"`.
    fn get_block(&self, x: i32, y: i32, z: i32) -> &str;

    /// Returns a reference to the chunk section at section coordinates `(sx, sy, sz)`, if present.
    fn get_section(&self, sx: i32, sy: i32, sz: i32) -> Option<&SectionStorage>;

    /// Checks if a section exists at section coordinates `(sx, sy, sz)`.
    fn contains_section(&self, sx: i32, sy: i32, sz: i32) -> bool {
        self.get_section(sx, sy, sz).is_some()
    }

    /// Returns the global bounding box `(min_block, max_block)` in block space, if bounded.
    fn block_bounds(&self) -> Option<(IVec3, IVec3)>;

    /// Returns the biome identifier at block coordinates `(x, y, z)`.
    fn get_biome(&self, x: i32, y: i32, z: i32) -> &str;
}

/// Mutation interface for setting blocks, modifying sections, and marking dirty regions.
pub trait VoxelWriter {
    /// Sets a block (with optional state encoded, e.g. "minecraft:oak_stairs[facing=north]")
    /// and optional biome at block coordinates `(x, y, z)`.
    fn set_block(&mut self, x: i32, y: i32, z: i32, state: &str, biome: Option<&str>);

    /// Ingests or replaces an entire 16x16x16 `SectionStorage` at section coordinates `(sx, sy, sz)`.
    fn set_section(&mut self, sx: i32, sy: i32, sz: i32, section: SectionStorage);

    /// Marks the specified section coordinate as modified/requiring remeshing.
    fn mark_section_dirty(&mut self, sx: i32, sy: i32, sz: i32);

    /// Sets the active world bounding box in block coordinates.
    fn set_bounds(&mut self, min_x: i32, min_y: i32, min_z: i32, size_x: i32, size_y: i32, size_z: i32);

    /// Clears all stored sections and resets dirty tracking.
    fn clear(&mut self);
}

/// Abstract provider capable of producing chunk sections on demand.
///
/// Implementors can wrap:
/// - Java Minecraft Anvil/MCA region files (`mtk-save`)
/// - Bedrock Edition LevelDB storage (`mtk-save`)
/// - Live WebSocket streaming client (`mtk-sync`)
/// - Mesh voxelization / inverse inference engine (`mtk-reconstruct`)
/// - Procedural terrain and noise generators
pub trait VoxelSource {
    /// Human-readable identifier or protocol name of this voxel source.
    fn source_name(&self) -> &str;

    /// Returns the total or estimated number of sections available in this source, if known.
    fn estimated_section_count(&self) -> Option<usize> {
        None
    }

    /// Checks whether the section at `(sx, sy, sz)` is present/available in this source.
    fn has_section(&self, sx: i32, sy: i32, sz: i32) -> bool;

    /// Loads or queries a single 16x16x16 `SectionStorage` at section coordinates `(sx, sy, sz)`.
    fn load_section(&mut self, sx: i32, sy: i32, sz: i32) -> Result<Option<SectionStorage>, VoxelError>;

    /// Bounding box `(min_section, max_section)` in section coordinates, if bounded.
    fn section_bounds(&self) -> Option<(IVec3, IVec3)> {
        None
    }

    /// Queries the biome identifier for the block at `(x, y, z)`, if the source provides biomes.
    fn sample_biome(&self, _x: i32, _y: i32, _z: i32) -> Option<&str> {
        None
    }
}

/// Ingests sections from a `VoxelSource` into a `VoxelWriter` target container.
///
/// If `sections` is `Some`, only those specified section coordinates will be queried and loaded.
/// If `sections` is `None` and the source provides `section_bounds()`, all sections within the bounds
/// will be ingested.
///
/// Returns the number of sections successfully ingested.
pub fn ingest_from_source<S: VoxelSource + ?Sized, W: VoxelWriter + ?Sized>(
    source: &mut S,
    target: &mut W,
    sections: Option<&[IVec3]>,
) -> Result<usize, VoxelError> {
    let mut count = 0;

    if let Some(list) = sections {
        for &sec_coord in list {
            if let Some(sec) = source.load_section(sec_coord.x, sec_coord.y, sec_coord.z)? {
                target.set_section(sec_coord.x, sec_coord.y, sec_coord.z, sec);
                count += 1;
            }
        }
    } else if let Some((min_sec, max_sec)) = source.section_bounds() {
        for sx in min_sec.x..=max_sec.x {
            for sy in min_sec.y..=max_sec.y {
                for sz in min_sec.z..=max_sec.z {
                    if source.has_section(sx, sy, sz) {
                        if let Some(sec) = source.load_section(sx, sy, sz)? {
                            target.set_section(sx, sy, sz, sec);
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(count)
}
