#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod engine;
pub mod rect_ops;
pub mod rules;
pub mod types;

pub use engine::{
    compute_block_cull_meta, derive_parametric_face_shapes, get_visible_face_directions,
    is_non_full_or_partial_block, parse_block_name_and_props, FaceCuller,
};
pub use rect_ops::{
    extract_face_occlusion_from_boxes, extract_quad_face_occlusion_rect,
    is_face_completely_occluded, is_fully_occluded, subtract_rect, subtract_rect_multi,
};
pub use rules::should_skip_rendering;
pub use types::{
    is_empty_rect, is_full_rect, BlockCullMeta, CullCategory, GlassCullMode, LeavesCullMode,
    EMPTY_FACE_RECT, FULL_FACE_RECT,
};

