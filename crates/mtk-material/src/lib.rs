//! # mtk-material
//!
//! High-performance, host-agnostic material name resolution, external alias mapping,
//! grid-based atlas unscrambling, and multi-threaded parallel UV re-addressing to libmtk AtlasAddressMaps.

pub mod biome;
pub mod error;
pub mod remap;
pub mod resolver;
pub mod types;

pub use biome::{
    blend_biome_colors, classify_tint_category, compute_mesh_biome_attributes,
    compute_mesh_biome_attributes_custom, get_biome_palette, get_colormap_uv,
    get_hardcoded_tint, hex_to_linear_rgba, hex_to_srgb, linear_to_srgb,
    srgb_to_linear, BiomePalette, BiomeResolver, CustomBiomeSettings,
    MeshBiomeAttributesResult, TintInfo, CANONICAL_BIOMES, HARDCODED_BLOCK_TINTS,
    TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED,
    TINT_TYPE_NONE, TINT_TYPE_WATER,
};
pub use error::MaterialError;
pub use remap::batch::{remap_mesh_multi_uvs_parallel, remap_mesh_uvs_parallel};
pub use remap::{
    is_quad_uv_diamond, remap_atlas_to_local, remap_local_to_atlas, remap_sprite_to_sprite,
    straighten_diamond_quad_uv,
};
pub use resolver::{
    clean_identifier, decode_grid_atlas_uv, remap_grid_atlas_uv_to_local, MaterialResolver,
};
pub use types::{GridAtlasSpec, MeshMultiUvRemapResult, MeshRemapResult, SourceUvSpace};
