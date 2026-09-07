#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod rect_ops;
pub mod types;

pub use rect_ops::{is_fully_occluded, subtract_rect, subtract_rect_multi};
pub use types::{CullCategory, GlassCullMode, LeavesCullMode, EMPTY_FACE_RECT, FULL_FACE_RECT};
