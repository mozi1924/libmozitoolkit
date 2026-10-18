#![forbid(unsafe_code)]

//! # mtk-material
//!
//! High-performance, host-agnostic material name resolution, external alias mapping,
//! grid-based atlas unscrambling, and multi-threaded parallel UV re-addressing to libmtk AtlasAddressMaps.

pub mod biome;
pub mod error;
pub mod properties;
pub mod remap;
pub mod resolver;
pub mod types;

pub use properties::{
    compute_mesh_material_props, compute_mesh_material_props_paletted, get_block_emission_strength,
    get_block_sticker_threshold, get_block_transmission_weight, get_material_props,
    is_thin_wall_block, is_transmissive_block, load_material_properties_json_replace,
    register_material_properties_json, reset_material_properties_to_default, EmissionConfig,
    MaterialPropertyConfig, MaterialPropertyRegistry, ThinWallConfig, TransmissiveConfig,
};

pub use biome::{
    blend_biome_colors, classify_tint_category, compute_mesh_biome_attributes,
    compute_mesh_biome_attributes_custom, compute_mesh_biome_attributes_paletted_flat,
    compute_mesh_biome_attributes_per_face, get_biome_palette, get_colormap_uv, get_hardcoded_tint,
    get_hardcoded_tint_hex, get_swamp_grass_color, get_unit_cube_tint_index, hex_to_linear_rgba,
    hex_to_srgb, linear_to_srgb, reapply_uniform_biome_tint, reapply_uniform_biome_tint_flat,
    sample_colormap_pixel, srgb_to_linear, BiomeColorSet, BiomePalette, BiomeResolver,
    BiomeTintFlatBuffers, CustomBiomeSettings, MeshBiomeAttributesResult, TintInfo,
    BLOCK_TINT_REGISTRY, CANONICAL_BIOMES, HARDCODED_BLOCK_TINTS, TINT_TYPE_DRY_FOLIAGE,
    TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED, TINT_TYPE_NONE, TINT_TYPE_WATER,
};
pub use error::MaterialError;
pub use remap::batch::{
    remap_mesh_multi_uvs_paletted_with_options, remap_mesh_multi_uvs_parallel,
    remap_mesh_multi_uvs_parallel_with_options,
};
pub use remap::routes::{reroute_mesh_animation, resolve_mesh_material_routes};
pub use remap::{
    is_quad_uv_diamond, remap_atlas_to_local, remap_local_to_atlas, remap_sprite_to_sprite,
    straighten_diamond_quad_uv,
};
pub use resolver::{
    clean_identifier, decode_grid_atlas_uv, extract_animation_metrics,
    remap_grid_atlas_uv_to_local, resolve_animated_sprite, resolve_routed_sprite, MaterialResolver,
};
pub use types::{
    GridAtlasSpec, MeshAnimationRerouteResult, MeshMaterialRoute, MeshMultiUvRemapResult,
    MeshPalettedUvRemapResult,
};
