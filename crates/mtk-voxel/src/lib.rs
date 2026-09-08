//! # mtk-voxel
//!
//! **mtk-voxel** provides Minecraft chunk section and world-level voxel storage,
//! physically-accurate fluid surface meshing, smooth ambient occlusion lighting,
//! biome color blending, and high-performance multi-threaded world mesh assembly.

pub mod biome;
pub mod fluid;
pub mod mesher;
pub mod storage;
pub mod types;

// Top-level re-exports
pub use biome::{get_biome_meta, get_colormap_uv, get_smoothed_biome_data, BiomeMeta};
pub use fluid::{
    calculate_corner_average, calculate_fluid_corner_heights, calculate_fluid_flow_vector,
    emit_fluid_geometry, get_fluid_base_height, get_fluid_side_uvs, get_fluid_top_uvs,
    sample_fluid_height, FluidType, MAX_FLUID_HEIGHT,
};
pub use mesher::{
    ao_level_to_brightness, calculate_face_ao, should_flip_quad_diagonal, vertex_ao, DeltaMesher,
    SectionMesher,
};
pub use storage::{
    block_index, crc32, crc32_update, extract_canonical_state_str, get_empty_section_crc,
    padded_index, PaddedVoxelArray, SectionStorage, VoxelStorage, EMPTY_SECTION_CRC, PADDED_SIZE,
    PADDED_VOLUME, SECTION_SIZE, SECTION_VOLUME,
};
pub use types::{CoordinateSystem, MesherConfig, VoxelError, WorldMeshBuildResult};

// Backward-compatibility module aliases
pub mod ao {
    pub use crate::mesher::ao::*;
}
pub mod crc {
    pub use crate::storage::crc::*;
}
pub mod delta_mesher {
    pub use crate::mesher::delta_mesher::*;
}
pub mod fluid_uv {
    pub use crate::fluid::uv::*;
}
pub mod world {
    pub use crate::storage::world::*;
}
