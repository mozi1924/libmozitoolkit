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
pub use mtk_model::baked::BakedModel;
pub use mtk_model::baker::ModelBaker;
pub use mtk_model::blockstate::BlockState;
pub use mtk_model::mesher::{SectionMesher, VoxelSection};

pub use mtk_model::model_json::BlockModelJson;
pub use mtk_model::obj::{mesh_to_obj_string, ModObjLoader, WavefrontObjParser};


