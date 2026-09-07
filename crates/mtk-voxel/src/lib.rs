//! # mtk-voxel
//!
//! **mtk-voxel** provides Minecraft chunk section and world-level voxel storage,
//! physically-accurate fluid surface meshing, smooth ambient occlusion lighting,
//! biome color blending, and high-performance multi-threaded world mesh assembly.

pub mod ao;
pub mod biome;
pub mod crc;
pub mod delta_mesher;
pub mod fluid;
pub mod fluid_uv;
pub mod mesher;
pub mod storage;
pub mod types;
pub mod world;

// Top-level re-exports
pub use ao::{ao_level_to_brightness, calculate_face_ao, should_flip_quad_diagonal, vertex_ao};
pub use biome::{get_biome_meta, get_colormap_uv, get_smoothed_biome_data, BiomeMeta};
pub use crc::{
    crc32, crc32_update, extract_canonical_state_str, get_empty_section_crc, EMPTY_SECTION_CRC,
};
pub use delta_mesher::DeltaMesher;
pub use fluid::{
    calculate_corner_average, calculate_fluid_corner_heights, calculate_fluid_flow_vector,
    emit_fluid_geometry, get_fluid_base_height, sample_fluid_height, FluidType, MAX_FLUID_HEIGHT,
};
pub use fluid_uv::{get_fluid_side_uvs, get_fluid_top_uvs};
pub use mesher::SectionMesher;
pub use storage::{
    block_index, padded_index, PaddedVoxelArray, SectionStorage, PADDED_SIZE, PADDED_VOLUME,
    SECTION_SIZE, SECTION_VOLUME,
};
pub use types::{CoordinateSystem, MesherConfig, VoxelError, WorldMeshBuildResult};
pub use world::VoxelStorage;
