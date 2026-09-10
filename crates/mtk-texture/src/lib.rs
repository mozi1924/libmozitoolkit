//! # mtk-texture
//!
//! Vanilla-style binary partitioning Stitcher, parallel image decoding,
//! PalettedPermutations baking, edge padding, and PBR-synced Atlas generator.

pub mod atlas;
pub mod error;
pub mod image;
pub mod standalone;
pub mod stitcher;

pub use atlas::{
    AtlasAddressMap, AtlasBuilder, AtlasBuilderConfig, AtlasChunkMeta, AtlasSpriteLocation,
    BakedAtlas, BakedAtlasChunk,
};
pub use error::TextureError;
pub use image::{
    apply_edge_clamping_padding, bake_paletted_permutation, extract_palette_colors,
    DecodedSprite, RgbaBuffer,
};
pub use standalone::{
    align_standalone_channels, ChannelData, ChannelType, StandaloneAlignResult,
    StandaloneAnimationMeta, StandaloneBuilder, StandaloneConfig, StandaloneFilePaths,
    StandaloneMapping, StandaloneResult, StandaloneTextureRecord, STANDALONE_FORMAT_VERSION,
};
pub use stitcher::{
    smallest_encompassing_power_of_two, StitchedAtlas, StitchedChunk, StitchedSlot, Stitcher,
    StitcherHolder, StitcherRegion,
};
