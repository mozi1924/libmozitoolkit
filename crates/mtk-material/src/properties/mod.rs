//! # Material Properties Catalog
//!
//! Evaluates physical shader parameters (emission, thin wall, transmission, sticker threshold)
//! for Minecraft blocks and textures in pure Rust.

pub mod batch;
pub mod emission;
pub mod thin_wall;
pub mod transmissive;

pub use batch::{compute_mesh_material_props, get_material_props};
pub use emission::get_block_emission_strength;
pub use thin_wall::is_thin_wall_block;
pub use transmissive::{
    get_block_sticker_threshold, get_block_transmission_weight, is_transmissive_block,
};
