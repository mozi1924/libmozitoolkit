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
pub use mtk_material as material;

use thiserror::Error;

/// Unified top-level error type for all libmtk operations.
#[derive(Debug, Error)]
pub enum MtkError {
    #[error("Model error: {0}")]
    Model(#[from] mtk_model::ModelError),

    #[error("Voxel error: {0}")]
    Voxel(#[from] mtk_voxel::VoxelError),

    #[error("Texture error: {0}")]
    Texture(#[from] mtk_texture::error::TextureError),

    #[error("Resource error: {0}")]
    Resource(#[from] mtk_resource::error::ResourceError),

    #[error("Material error: {0}")]
    Material(#[from] mtk_material::MaterialError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
}

pub use mtk_resource::{
    coordinate_random, extract_block_name, get_face_tangents, AnimationFrame, AnimationMetadata,
    AtlasDefinition, AtlasSource, BlockMatch, ConnectLogic, CtmMethod, CtmRule, CtmSolver,
    CtmSymmetry, DirectoryPack, DiscoveredSprite, MemoryPack, PbrCompanions, ResourceLocation,
    ResourcePack, ResourcePackStack, TextureMetadata, CTM_47_LOOKUP, OVERLAY_17_LOOKUP,
};
pub use mtk_texture::{
    align_standalone_channels, AtlasAddressMap, AtlasBuilder, AtlasBuilderConfig, AtlasChunkMeta,
    AtlasSpriteLocation, BakedAtlas, BakedAtlasChunk, ChannelData, ChannelType, DecodedSprite,
    RgbaBuffer, SpriteKind, StandaloneAlignResult, StandaloneAnimationMeta, StandaloneBuilder,
    StandaloneConfig, StandaloneFilePaths, StandaloneMapping, StandaloneResult,
    StandaloneTextureRecord, Stitcher, STANDALONE_FORMAT_VERSION,
};
pub use mtk_material::{
    clean_identifier, decode_grid_atlas_uv, is_quad_uv_diamond, remap_atlas_to_local,
    remap_grid_atlas_uv_to_local, remap_local_to_atlas, remap_mesh_multi_uvs_parallel,
    remap_mesh_uvs_parallel, remap_sprite_to_sprite, straighten_diamond_quad_uv,
    GridAtlasSpec, MaterialResolver, MeshMultiUvRemapResult, MeshRemapResult, SourceUvSpace,
};
pub use mtk_core::geometry::{
    mc_local_to_centered_z_up, mc_world_to_z_up, Aabb2d, Aabb3d, Quad,
};
pub mod pipeline;
pub use pipeline::{
    process_mesh, MeshPipelineConfig, MeshProcessStats, ProcessMeshOutput, ResolvedMaterialInfo,
};
pub mod prebake;
pub use prebake::{
    prebake_all_models, precompile_all_assets, CacheManifest, PrecompileConfig, PrecompileResult,
    ASSET_CACHE_FORMAT_VERSION,
};
pub use mtk_model::baker::{BakedModel, BakedModelDatabase, ModelBaker};
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
pub use mtk_voxel::protocol;
pub use mtk_voxel::protocol::{
    decode_packet, encode_full_sync_request, encode_repair_requests, encode_sync_config,
    DeltaChange, ManifestSectionEntry, Packet, PacketType, ProtocolError, StreamStatus,
};
#[cfg(feature = "sync")]
pub use mtk_voxel::sync::{LiveSyncSession, SyncEvent};


