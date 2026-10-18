//! # High-Throughput Batch Material Properties Evaluator

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use super::registry::with_global_registry;

/// Evaluates packed material properties `[emission, thin_wall, transmission, sticker_threshold]` for a single block/texture.
#[inline]
pub fn get_material_props(block_name: &str, texture_name: Option<&str>) -> [f32; 4] {
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
                    let blk_name = block_names
                        .and_then(|names| names.get(i).map(|s| s.as_str()))
                        .unwrap_or("");
                    reg.get_material_props(blk_name, Some(tex_key))
                })
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut out = Vec::with_capacity(len);
            for i in 0..len {
                let tex_key = &face_texture_keys[i];
                let blk_name = block_names
                    .and_then(|names| names.get(i).map(|s| s.as_str()))
                    .unwrap_or("");
                out.push(reg.get_material_props(blk_name, Some(tex_key)));
            }
            out
        }
    })
}

/// Evaluate packed material properties for palette + per-face indices.
///
/// The palette entries are resolved once, so a mesh with a handful of unique
/// textures does not perform a registry lookup per face. `block_names` is
/// optional and, when supplied, is still resolved per face.
pub fn compute_mesh_material_props_paletted(
    palette: &[String],
    face_indices: &[u32],
    block_names: Option<&[String]>,
) -> Vec<[f32; 4]> {
    with_global_registry(|reg| {
        let palette_props: Vec<[f32; 4]> = palette
            .iter()
            .map(|key| reg.get_material_props("", Some(key)))
            .collect();

        #[cfg(feature = "parallel")]
        {
            (0..face_indices.len())
                .into_par_iter()
                .map(|i| {
                    let key = face_indices
                        .get(i)
                        .and_then(|idx| palette.get(*idx as usize))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    match block_names.and_then(|names| names.get(i)) {
                        Some(block) => reg.get_material_props(block, Some(key)),
                        None => face_indices
                            .get(i)
                            .and_then(|idx| palette_props.get(*idx as usize).copied())
                            .unwrap_or([0.0; 4]),
                    }
                })
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            let mut out = Vec::with_capacity(face_indices.len());
            for i in 0..face_indices.len() {
                let key = face_indices
                    .get(i)
                    .and_then(|idx| palette.get(*idx as usize))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let props = match block_names.and_then(|names| names.get(i)) {
                    Some(block) => reg.get_material_props(block, Some(key)),
                    None => face_indices
                        .get(i)
                        .and_then(|idx| palette_props.get(*idx as usize).copied())
                        .unwrap_or([0.0; 4]),
                };
                out.push(props);
            }
            out
        }
    })
}
