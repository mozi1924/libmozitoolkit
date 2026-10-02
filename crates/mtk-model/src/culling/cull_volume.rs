//! Re-exports volume occlusion clipping and mesh sanitization from `mtk_cull`.
//!
//! Preserved for backward-compatibility; geometric clipping and sanitization implementations
//! reside in `mtk_cull::geometry::volume_cull` and `mtk_cull::MeshSanitizer`.

pub use mtk_cull::{clip_face_excluding_hidden_volume, ClippedQuadPiece, MeshSanitizer};
