//! Minecraft Biome Palettes, Model Tintindex Resolution, and Mesh Attribute Generation.

pub mod batch;
pub mod canonical_data;
pub mod hardcoded;
pub mod palettes;
pub mod resolver;

pub use batch::{
    compute_mesh_biome_attributes, compute_mesh_biome_attributes_custom,
    compute_mesh_biome_attributes_per_face, reapply_uniform_biome_tint,
    reapply_uniform_biome_tint_flat, BiomeColorSet, BiomeTintFlatBuffers, CustomBiomeSettings,
    MeshBiomeAttributesResult,
};
pub use hardcoded::{
    classify_tint_category, get_hardcoded_tint, get_hardcoded_tint_hex, get_swamp_grass_color,
    get_unit_cube_tint_index, BLOCK_TINT_REGISTRY, HARDCODED_BLOCK_TINTS, TINT_TYPE_DRY_FOLIAGE,
    TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED, TINT_TYPE_NONE, TINT_TYPE_WATER,
};
pub use palettes::{
    blend_biome_colors, get_biome_palette, get_colormap_uv, hex_to_linear_rgba, hex_to_srgb,
    linear_to_srgb, sample_colormap_pixel, srgb_to_linear, BiomePalette, CANONICAL_BIOMES,
};
pub use resolver::{BiomeResolver, TintInfo};
