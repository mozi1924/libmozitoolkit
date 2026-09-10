#[cfg(feature = "parallel")]
use rayon::prelude::*;
use mtk_texture::AtlasAddressMap;

use crate::mineways::decode_mineways_uv;
use crate::remap::remap_local_to_atlas;
use crate::resolver::MaterialResolver;
use crate::types::{ImporterOrigin, MeshRemapResult};

/// Parallel batch remapper for entire mesh UV layers and face material assignments.
pub fn remap_mesh_uvs_parallel(
    uvs: &mut [[f32; 2]],
    face_materials: &[String],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    origin: ImporterOrigin,
    mineways_atlas_size: Option<(u32, u32)>,
) -> MeshRemapResult {
    let face_count = face_materials.len();
    if face_count != face_loop_ranges.len() {
        return MeshRemapResult {
            face_count,
            loop_count: uvs.len(),
            face_chunk_ids: vec![0; face_count],
            face_texture_ids: vec![0; face_count],
            unmapped_faces: face_count,
        };
    }

    // 1. Resolve material names to sprite locations ahead of loop transforms
    let (mw_w, mw_h) = mineways_atlas_size.unwrap_or((1024, 1024));
    let uvs_raw_addr = uvs.as_mut_ptr() as usize;
    let total_uv_len = uvs.len();

    let map_face = move |face_idx: usize| -> (u16, u32, bool) {
        let uvs_ptr = uvs_raw_addr as *mut [f32; 2];
        let (start, count) = face_loop_ranges[face_idx];
        let start_idx = start as usize;
        let end_idx = start_idx + (count as usize);

        if start_idx >= total_uv_len || end_idx > total_uv_len || count == 0 {
            return (0, 0, false);
        }

        let mat_name = &face_materials[face_idx];
        let is_mineways_atlas = mineways_atlas_size.is_some() && crate::mineways::is_mineways_atlas_name(mat_name);

        if is_mineways_atlas {
            // Calculate center UV of the face to decode Mineways swatch
            let mut sum_u = 0.0f32;
            let mut sum_v = 0.0f32;
            for i in start_idx..end_idx {
                unsafe {
                    let p = uvs_ptr.add(i);
                    sum_u += (*p)[0];
                    sum_v += (*p)[1];
                }
            }
            let avg_u = sum_u / (count as f32);
            let avg_v = sum_v / (count as f32);

            if let Some((_, sprite_loc, _)) =
                MaterialResolver::resolve_mineways_face(avg_u, avg_v, mw_w, mw_h, address_map)
            {
                // Remap each loop UV from Mineways atlas -> local -> target atlas
                for i in start_idx..end_idx {
                    unsafe {
                        let p = uvs_ptr.add(i);
                        let [u_in, v_in] = *p;
                        let (_, _, local_uv) = decode_mineways_uv(u_in, v_in, mw_w, mw_h);
                        let target_uv = remap_local_to_atlas(local_uv[0], local_uv[1], sprite_loc);
                        *p = target_uv;
                    }
                }
                return (sprite_loc.chunk_id, sprite_loc.texture_id, true);
            }
            return (0, 0, false);
        }

        // Normal material path
        if let Some((_, sprite_loc)) = MaterialResolver::resolve(mat_name, origin, address_map) {
            for i in start_idx..end_idx {
                unsafe {
                    let p = uvs_ptr.add(i);
                    let [u_in, v_in] = *p;
                    let target_uv = remap_local_to_atlas(u_in, v_in, sprite_loc);
                    *p = target_uv;
                }
            }
            (sprite_loc.chunk_id, sprite_loc.texture_id, true)
        } else {
            (0, 0, false)
        }
    };

    #[cfg(feature = "parallel")]
    let face_results: Vec<(u16, u32, bool)> = (0..face_count).into_par_iter().map(map_face).collect();

    #[cfg(not(feature = "parallel"))]
    let face_results: Vec<(u16, u32, bool)> = (0..face_count).into_iter().map(map_face).collect();

    // Populate final results
    let mut face_chunk_ids = vec![0u16; face_count];
    let mut face_texture_ids = vec![0u32; face_count];
    let mut mapped_faces = 0usize;
    for (i, (chunk_id, tex_id, success)) in face_results.into_iter().enumerate() {
        face_chunk_ids[i] = chunk_id;
        face_texture_ids[i] = tex_id;
        if success {
            mapped_faces += 1;
        }
    }

    MeshRemapResult {
        face_count,
        loop_count: total_uv_len,
        face_chunk_ids,
        face_texture_ids,
        unmapped_faces: face_count.saturating_sub(mapped_faces),
    }
}
