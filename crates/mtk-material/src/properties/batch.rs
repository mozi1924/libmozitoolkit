//! # High-Throughput Batch Material Properties Evaluator

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use super::registry::with_global_registry;

/// Evaluates packed material properties `[emission, thin_wall, transmission, sticker_threshold]` for a single block/texture.
#[inline]
pub fn get_material_props(
    block_name: &str,
    texture_name: Option<&str>,
) -> [f32; 4] {
    with_global_registry(|reg| reg.get_material_props(block_name, texture_name))
}

/// Evaluates packed material properties for a slice of face texture keys and optional block names.
/// Uses Rayon parallel iteration when feature is active, acquiring a single read lock upfront.
pub fn compute_mesh_material_props(
    face_texture_keys: &[String],
    block_names: Option<&[String]>,
) -> Vec<[f32; 4]> {
    let len = face_texture_keys.len();

    with_global_registry(|reg| {
        #[cfg(feature = "parallel")]
        {
            (0..len)
                .into_par_iter()
                .map(|i| {
                    let tex_key = &face_texture_keys[i];
                    let blk_name = block_names.and_then(|names| names.get(i).map(|s| s.as_str())).unwrap_or("");
                    reg.get_material_props(blk_name, Some(tex_key))
                })
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut out = Vec::with_capacity(len);
            for i in 0..len {
                let tex_key = &face_texture_keys[i];
                let blk_name = block_names.and_then(|names| names.get(i).map(|s| s.as_str())).unwrap_or("");
                out.push(reg.get_material_props(blk_name, Some(tex_key)));
            }
            out
        }
    })
}
