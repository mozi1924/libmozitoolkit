//! # libmtk
//!
//! **libmtk** is the high-performance, host-agnostic, pure Rust core engine for MoziToolKit 2.0.
//!
//! It provides:
//! - Core geometry buffers and Minecraft 6-direction primitives (`mtk_core`)
//! - Block occlusion states and 2D rectangle difference clipping (`mtk_cull`)
//! - BlockState string parsing and Block Model JSON data structures (`mtk_model`)

pub use mtk_core as core;
pub use mtk_cull as cull;
pub use mtk_model as model;

// Convenient top-level re-exports
pub use mtk_core::direction::{DirMask, Direction};
pub use mtk_core::geometry::{Aabb2d, Aabb3d, Quad};
pub use mtk_core::mesh::MeshData;
pub use mtk_cull::rect_ops::{is_fully_occluded, subtract_rect, subtract_rect_multi};
pub use mtk_cull::types::CullCategory;
pub use mtk_model::baked::BakedModel;
pub use mtk_model::baker::ModelBaker;
pub use mtk_model::blockstate::BlockState;
pub use mtk_model::model_json::BlockModelJson;
pub use mtk_model::obj::{ModObjLoader, WavefrontObjParser};
