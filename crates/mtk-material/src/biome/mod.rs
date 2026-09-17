//! Minecraft Biome Palettes, Model Tintindex Resolution, and Mesh Attribute Generation.

pub mod batch;
pub mod hardcoded;
pub mod palettes;
pub mod resolver;

pub use batch::{
    compute_mesh_biome_attributes, compute_mesh_biome_attributes_custom,
    CustomBiomeSettings, MeshBiomeAttributesResult,
};
pub use hardcoded::{
    classify_tint_category, get_hardcoded_tint, HARDCODED_BLOCK_TINTS,
    TINT_TYPE_DRY_FOLIAGE, TINT_TYPE_FOLIAGE, TINT_TYPE_GRASS, TINT_TYPE_HARDCODED,
    TINT_TYPE_NONE, TINT_TYPE_WATER,
};
pub use palettes::{
    blend_biome_colors, get_biome_palette, get_colormap_uv, hex_to_linear_rgba, hex_to_srgb,
    linear_to_srgb, srgb_to_linear, BiomePalette, CANONICAL_BIOMES,
};
pub use resolver::{BiomeResolver, TintInfo};
