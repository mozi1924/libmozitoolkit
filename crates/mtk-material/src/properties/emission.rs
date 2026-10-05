//! # Block and Texture Emission Evaluation

use std::collections::HashMap;
use super::registry::with_global_registry;

/// Evaluates emission strength for a block name, optional state properties, and optional texture name.
pub fn get_block_emission_strength(
    block_name: &str,
    properties: Option<&HashMap<String, String>>,
    texture_name: Option<&str>,
) -> f32 {
    with_global_registry(|reg| reg.get_block_emission_strength(block_name, properties, texture_name))
}
