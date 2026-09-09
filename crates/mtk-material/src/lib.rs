//! # mtk-material
//!
//! Material name resolution, multi-importer cleaning (Mineways, jmc2obj, Ice-Cube),
//! Mineways atlas unscrambling, and high-performance parallel UV re-addressing to libmtk AtlasAddressMaps.

pub mod error;
pub mod icecube;
pub mod jmc2obj;
pub mod mineways;
pub mod remap;
pub mod resolver;
pub mod types;

pub use error::MaterialError;
pub use icecube::{clean_icecube_name, resolve_icecube_candidates};
pub use jmc2obj::{clean_jmc2obj_name, resolve_jmc2obj_candidates};
pub use mineways::{
    decode_mineways_uv, is_mineways_atlas_name, lookup_swatch, remap_mineways_atlas_uv_to_local,
};
pub use remap::batch::remap_mesh_uvs_parallel;
pub use remap::{
    is_quad_uv_diamond, remap_atlas_to_local, remap_local_to_atlas, remap_sprite_to_sprite,
    straighten_diamond_quad_uv,
};
pub use resolver::{detect_importer_origin, MaterialResolver};
pub use types::{ImporterOrigin, MeshRemapResult, SourceUvSpace};
