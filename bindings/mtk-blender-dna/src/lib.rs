//! # mtk-blender-dna
//!
//! Blender DNA direct raw memory pointer zero-copy adapter for `libmtk`.
//!
//! Provides dedicated zero-copy raw memory pointer injection specifically tailored
//! for Blender DNA CustomData memory layouts, with bounds checking and memory safety guards.

pub mod error;
pub mod guard;
pub mod mesh;
pub mod point_cloud;

pub use error::{DnaError, DnaResult};
pub use guard::{validate_aligned_pointer_and_bounds, validate_pointer_and_bounds};
pub use mesh::{
    copy_colors_to_ptr, copy_indices_to_ptr, copy_loop_uvs_to_ptr, copy_normals_to_ptr,
    copy_positions_to_ptr, copy_quad_indices_to_ptr,
};
pub use point_cloud::{
    copy_point_cloud_biome_indices_to_ptr, copy_point_cloud_block_x_to_ptr,
    copy_point_cloud_block_y_to_ptr, copy_point_cloud_block_z_to_ptr,
    copy_point_cloud_light_levels_to_ptr, copy_point_cloud_positions_to_ptr,
    copy_point_cloud_state_indices_to_ptr, copy_point_cloud_to_ptrs,
};
