//! # Dielectric Transmission and Refraction Evaluation

use super::registry::with_global_registry;

/// Checks if a block or texture is a transmissive / refractive dielectric (glass, water, ice, etc.).
pub fn is_transmissive_block(block_name: &str, texture_name: Option<&str>) -> bool {
    with_global_registry(|reg| reg.is_transmissive_block(block_name, texture_name))
}

/// Evaluates transmission weight (1.0 for transmissive, 0.0 otherwise).
pub fn get_block_transmission_weight(block_name: &str, texture_name: Option<&str>) -> f32 {
    with_global_registry(|reg| reg.get_block_transmission_weight(block_name, texture_name))
}

/// Evaluates alpha sticker threshold (0.55 for glass, 0.95 for water/ice/slime/honey).
pub fn get_block_sticker_threshold(block_name: &str, texture_name: Option<&str>) -> f32 {
    with_global_registry(|reg| reg.get_block_sticker_threshold(block_name, texture_name))
}
