//! # Meshing Shading, Material, CTM, and Biome Resolver
//!
//! Encapsulates material resolution, CTM solver querying, Atlas UV lookup,
//! and Biome color sampling for voxel meshing, decoupling material logic
//! from voxel traversal.

pub mod anim;
pub mod palette;
pub mod resolver;
pub mod types;

pub use anim::{extract_anim_metrics, lookup_animated_from_cache};
pub use palette::{build_palette_meshing_data, build_palette_meshing_data_from_models};
pub use resolver::{resolve_model_face_shading, resolve_unit_cube_face_shading, sample_biome_tint};
pub use types::{
    ModelFaceBuckets, ModelSource, PaletteMeshingData, PreResolvedFace, PreResolvedModelFace,
    ResolvedFaceShading,
};
