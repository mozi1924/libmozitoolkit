//! # High-Throughput Batch Material Properties Evaluator

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use super::emission::get_block_emission_strength;
use super::thin_wall::is_thin_wall_block;
use super::transmissive::{get_block_sticker_threshold, get_block_transmission_weight};

/// Evaluates packed material properties `[emission, thin_wall, transmission, sticker_threshold]` for a single block/texture.
#[inline]
pub fn get_material_props(
    block_name: &str,
    texture_name: Option<&str>,
) -> [f32; 4] {
    let emission = get_block_emission_strength(block_name, None, texture_name);
    let thin_wall = if is_thin_wall_block(block_name, texture_name) { 1.0 } else { 0.0 };
    let transmission = get_block_transmission_weight(block_name, texture_name);
    let sticker_threshold = get_block_sticker_threshold(block_name, texture_name);
    [emission, thin_wall, transmission, sticker_threshold]
}

/// Evaluates packed material properties for a slice of face texture keys and optional block names.
/// Uses Rayon parallel iteration when feature is active.
pub fn compute_mesh_material_props(
    face_texture_keys: &[String],
    block_names: Option<&[String]>,
) -> Vec<[f32; 4]> {
    let len = face_texture_keys.len();

    #[cfg(feature = "parallel")]
    {
        (0..len)
            .into_par_iter()
            .map(|i| {
                let tex_key = &face_texture_keys[i];
                let blk_name = block_names.and_then(|names| names.get(i).map(|s| s.as_str())).unwrap_or("");
                get_material_props(blk_name, Some(tex_key))
            })
            .collect()
    }

    #[cfg(not(feature = "parallel"))]
    {
        let mut out = Vec::with_capacity(len);
        for i in 0..len {
            let tex_key = &face_texture_keys[i];
            let blk_name = block_names.and_then(|names| names.get(i).map(|s| s.as_str())).unwrap_or("");
            out.push(get_material_props(blk_name, Some(tex_key)));
        }
        out
    }
}
