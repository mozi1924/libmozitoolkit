//! # libmtk
//!
//! **libmtk** is the high-performance, host-agnostic, pure Rust core engine for MoziToolKit 2.0.
//!
//! It provides:
//! - Core geometry buffers and Minecraft 6-direction primitives (`mtk_core`)
//! - Block occlusion states and 2D rectangle difference clipping (`mtk_cull`)
//! - BlockState string parsing and Block Model JSON data structures (`mtk_model`)

pub use mtk_core as core;
pub use mtk_core::constants;
pub use mtk_cull as cull;
pub use mtk_model as model;
pub use mtk_voxel as voxel;
pub use mtk_resource as resource;
pub use mtk_texture as texture;

// Convenient top-level re-exports
pub use mtk_resource::{
    AnimationFrame, AnimationMetadata, AtlasDefinition, AtlasSource, DirectoryPack,
    DiscoveredSprite, MemoryPack, PbrCompanions, ResourceLocation, ResourcePack, ResourcePackStack,
    TextureMetadata,
};
pub use mtk_texture::{
    AtlasAddressMap, AtlasBuilder, AtlasBuilderConfig, AtlasChunkMeta, AtlasSpriteLocation,
    BakedAtlas, BakedAtlasChunk, DecodedSprite, RgbaBuffer, Stitcher,
};
pub use mtk_core::direction::{DirMask, Direction};
pub use mtk_core::geometry::{Aabb2d, Aabb3d, Quad};
pub use mtk_core::mesh::MeshData;
pub use mtk_cull::engine::{
    compute_block_cull_meta, derive_parametric_face_shapes, get_visible_face_directions,
    is_non_full_or_partial_block, parse_block_name_and_props, FaceCuller,
};
pub use mtk_cull::rect_ops::{
    extract_face_occlusion_from_boxes, extract_quad_face_occlusion_rect,
    is_face_completely_occluded, is_fully_occluded, subtract_rect, subtract_rect_multi,
};
pub use mtk_cull::rules::should_skip_rendering;
pub use mtk_cull::types::{
    BlockCullMeta, CullCategory, GlassCullMode, LeavesCullMode, EMPTY_FACE_RECT, FULL_FACE_RECT,
};
pub use mtk_model::baked::BakedModel;
pub use mtk_model::baker::ModelBaker;
pub use mtk_model::blockstate::BlockState;
pub use mtk_model::model_json::BlockModelJson;
pub use mtk_model::obj::{mesh_to_obj_string, ModObjLoader, WavefrontObjParser};
pub use mtk_voxel::ao::{ao_level_to_brightness, calculate_face_ao, should_flip_quad_diagonal};
pub use mtk_voxel::biome::{get_biome_meta, get_colormap_uv, get_smoothed_biome_data};
pub use mtk_voxel::crc::{crc32, get_empty_section_crc, EMPTY_SECTION_CRC};
pub use mtk_voxel::delta_mesher::DeltaMesher;
pub use mtk_voxel::fluid::{calculate_fluid_corner_heights, calculate_fluid_flow_vector, FluidType};
pub use mtk_voxel::mesher::SectionMesher;
pub use mtk_voxel::storage::{PaddedVoxelArray, SectionStorage};
pub use mtk_voxel::types::{CoordinateSystem, MesherConfig, WorldMeshBuildResult};
pub use mtk_voxel::world::VoxelStorage;


