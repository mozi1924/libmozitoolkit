//! # mtk-save
//!
//! **mtk-save** provides high-performance Minecraft Java Edition save (Anvil MCA)
//! decoding and spatial ingestion for MoziToolKit.
//!
//! Key components:
//! - [`LevelData`]: `level.dat` metadata deserializer.
//! - [`RegionFile`]: Anvil `.mca` 1024-chunk location table parser and direct-seek reader.
//! - [`ChunkParser`]: 1.18+ chunk section bit-unpacker for `block_states` and biomes.
//! - [`AnvilWorldSource`]: `VoxelSource` implementation for on-demand chunk streaming.
//! - [`SaveLoader`]: High-level bounding box slicer and directory discovery engine.

pub mod chunk;
pub mod error;
pub mod level;
pub mod loader;
pub mod nbt;
pub mod region;
pub mod source;

pub use chunk::{format_canonical_blockstate, ChunkParser};
pub use error::SaveError;
pub use level::LevelData;
pub use loader::SaveLoader;
pub use region::{
    ChunkLocation, RegionFile, REGION_CHUNKS_AXIS, REGION_HEADER_SIZE, REGION_TOTAL_CHUNKS,
    SECTOR_SIZE,
};
pub use source::AnvilWorldSource;
