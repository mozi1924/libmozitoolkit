//! # mtk-material
//!
//! High-performance, host-agnostic material name resolution, external alias mapping,
//! grid-based atlas unscrambling, and multi-threaded parallel UV re-addressing to libmtk AtlasAddressMaps.

pub mod error;
pub mod remap;
pub mod resolver;
pub mod types;

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
