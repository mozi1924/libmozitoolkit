#![forbid(unsafe_code)]
//! # mtk-save
//!
//! **mtk-save** provides high-performance Minecraft Java Edition save (Anvil MCA)
//! decoding and spatial ingestion for MoziToolKit.
//!
//! Key components:
//! - [`LevelData`]: `level.dat` metadata deserializer.
//! - [`RegionFile`]: Anvil `.mca` 1024-chunk location table parser and direct-seek reader.
//! - [`ChunkParser`]: 1.18+ chunk section bit-unpacker for `block_states` and biomes.
//! - [`SaveLoader`]: High-level bounding box slicer and directory discovery engine.

pub mod chunk;
pub mod error;
pub mod level;
pub mod loader;
pub mod nbt;
pub mod region;

pub use chunk::ChunkParser;
pub use error::SaveError;
pub use level::LevelData;
pub use loader::SaveLoader;
pub use region::RegionFile;
