#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod attributes;
pub mod direction;
pub mod geometry;
pub mod mesh;

pub use attributes::{FaceAttributes, LightLevel, MaterialSlotId, TintIndex};
pub use direction::{DirMask, Direction};
pub use geometry::{Aabb2d, Aabb3d, Quad};
pub use mesh::MeshData;

// Re-export glam types for convenience
pub use glam::{IVec3, Vec2, Vec3, Vec4};
