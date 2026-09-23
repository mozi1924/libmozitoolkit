#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod attributes;
pub mod constants;
pub mod direction;
pub mod extrude;
pub mod geometry;
pub mod mesh;
pub mod subdivide;
pub mod uv;

pub use attributes::{
    AttributeData, AttributeDomain, FaceAttributes, LightLevel, MaterialSlotId, MeshAttribute,
    TintIndex,
};
pub use constants::concurrency;
pub use direction::{DirMask, Direction};
pub use extrude::{
    cellular_noise_3d, generate_extrude_heights, perlin_noise_3d, repair_extruded_side_uv,
    ExtrudeNoiseType, ExtrudeUvMode,
};
pub use geometry::{mc_local_to_centered_z_up, mc_world_to_z_up, Aabb2d, Aabb3d, Quad};
pub use mesh::MeshData;
pub use subdivide::{
    adaptive_pixel_split_mesh, calculate_face_target_grid, interpolate_bilinear_2d,
    interpolate_bilinear_3d, interpolate_bilinear_4d, weld_mesh_vertices,
};

// Re-export glam types for convenience
pub use glam::{IVec3, Vec2, Vec3, Vec4};


