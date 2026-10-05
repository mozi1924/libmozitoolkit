//! # Foliage and Vegetation Thin Wall Evaluation

use super::registry::with_global_registry;

/// Checks if a block or texture represents thin-wall foliage or vegetation.
pub fn is_thin_wall_block(block_name: &str, texture_name: Option<&str>) -> bool {
    with_global_registry(|reg| reg.is_thin_wall_block(block_name, texture_name))
}
