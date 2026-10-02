//! Re-exports volume occlusion clipping from `mtk_cull`.
//!
//! Preserved for backward-compatibility; geometric clipping implementation
//! resides in `mtk_cull::geometry::volume_cull`.

pub use mtk_cull::{clip_face_excluding_hidden_volume, ClippedQuadPiece};
