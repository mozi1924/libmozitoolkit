#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod attributes;
pub mod constants;
pub mod direction;
pub mod geometry;
pub mod mesh;

pub use attributes::{FaceAttributes, LightLevel, MaterialSlotId, TintIndex};
pub use constants::concurrency;
pub use direction::{DirMask, Direction};
pub use geometry::{mc_local_to_blender, mc_world_to_blender, Aabb2d, Aabb3d, Quad};
pub use mesh::MeshData;

// Re-export glam types for convenience
pub use glam::{IVec3, Vec2, Vec3, Vec4};

