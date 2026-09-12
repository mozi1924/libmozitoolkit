use std::collections::HashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use mtk_texture::AtlasAddressMap;

use crate::remap::remap_local_to_atlas;
use crate::resolver::decode_grid_atlas_uv;
use crate::resolver::MaterialResolver;
use crate::types::{GridAtlasSpec, MeshRemapResult};

/// Parallel batch remapper for entire mesh UV layers and face material assignments.
pub fn remap_mesh_uvs_parallel(
    uvs: &mut [[f32; 2]],
    face_materials: &[String],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
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
        let is_grid_atlas = grid_atlas_spec.map_or(false, |spec| spec.matches_atlas_name(mat_name));

        if is_grid_atlas {
            let spec = grid_atlas_spec.unwrap();
            // Calculate center UV of the face to decode grid atlas swatch
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
                MaterialResolver::resolve_grid_atlas_face(avg_u, avg_v, spec, aliases, address_map)
            {
                // Remap each loop UV from grid atlas -> local -> target atlas
                for i in start_idx..end_idx {
                    unsafe {
                        let p = uvs_ptr.add(i);
                        let [u_in, v_in] = *p;
                        let (_, local_uv) = decode_grid_atlas_uv(u_in, v_in, spec);
                        let target_uv = remap_local_to_atlas(local_uv[0], local_uv[1], sprite_loc);
                        *p = target_uv;
                    }
                }
                return (sprite_loc.chunk_id, sprite_loc.texture_id, true);
            }
            return (0, 0, false);
        }

        // Normal material path
        if let Some((_, sprite_loc)) = MaterialResolver::resolve(mat_name, aliases, address_map) {
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

/// Parallel batch remapper producing both Atlas UVs (chunk-mapped) and Local UVs ([0..1] normalized),
/// along with per-face UV mode routing flags and overlay detection.
pub fn remap_mesh_multi_uvs_parallel(
    source_uvs: &[[f32; 2]],
    face_materials: &[String],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
) -> crate::types::MeshMultiUvRemapResult {
    let face_count = face_materials.len();
    let total_uv_len = source_uvs.len();

    if face_count != face_loop_ranges.len() {
        return crate::types::MeshMultiUvRemapResult {
            face_count,
            loop_count: total_uv_len,
            atlas_uvs: source_uvs.to_vec(),
            local_uvs: source_uvs.to_vec(),
            face_chunk_ids: vec![0; face_count],
            face_texture_ids: vec![0; face_count],
            face_uv_modes: vec![0; face_count],
            face_is_overlay: vec![false; face_count],
            unmapped_faces: face_count,
        };
    }

    // Structure holding per-face computed results
    struct FaceOut {
        chunk_id: u16,
        texture_id: u32,
        uv_mode: u8,
        is_overlay: bool,
        success: bool,
    }

    let mut atlas_uvs = vec![[0.0f32, 0.0f32]; total_uv_len];
    let mut local_uvs = vec![[0.0f32, 0.0f32]; total_uv_len];

    let atlas_uvs_ptr = atlas_uvs.as_mut_ptr() as usize;
    let local_uvs_ptr = local_uvs.as_mut_ptr() as usize;

    let map_face = move |face_idx: usize| -> FaceOut {
        let a_ptr = atlas_uvs_ptr as *mut [f32; 2];
        let l_ptr = local_uvs_ptr as *mut [f32; 2];
        let (start, count) = face_loop_ranges[face_idx];
        let start_idx = start as usize;
        let end_idx = start_idx + (count as usize);

        if start_idx >= total_uv_len || end_idx > total_uv_len || count == 0 {
            return FaceOut {
                chunk_id: 0,
                texture_id: 0,
                uv_mode: 0,
                is_overlay: false,
                success: false,
            };
        }

        let mat_name = &face_materials[face_idx];
        let is_overlay = mat_name.contains("overlay") || mat_name.contains("grass_side_overlay");
        let is_grid_atlas = grid_atlas_spec.map_or(false, |spec| spec.matches_atlas_name(mat_name));

        if is_grid_atlas {
            let spec = grid_atlas_spec.unwrap();
            let mut sum_u = 0.0f32;
            let mut sum_v = 0.0f32;
            for i in start_idx..end_idx {
                let [u_in, v_in] = source_uvs[i];
                sum_u += u_in;
                sum_v += v_in;
            }
            let avg_u = sum_u / (count as f32);
            let avg_v = sum_v / (count as f32);

            if let Some((_, sprite_loc, _)) =
                MaterialResolver::resolve_grid_atlas_face(avg_u, avg_v, spec, aliases, address_map)
            {
                let uv_mode = if is_overlay { 3 } else { 0 };
                for i in start_idx..end_idx {
                    let [u_in, v_in] = source_uvs[i];
                    let (_, local_uv) = decode_grid_atlas_uv(u_in, v_in, spec);
                    let target_atlas_uv = remap_local_to_atlas(local_uv[0], local_uv[1], sprite_loc);
                    unsafe {
                        *a_ptr.add(i) = target_atlas_uv;
                        *l_ptr.add(i) = local_uv;
                    }
                }
                return FaceOut {
                    chunk_id: sprite_loc.chunk_id,
                    texture_id: sprite_loc.texture_id,
                    uv_mode,
                    is_overlay,
                    success: true,
                };
            }
            // Unresolved grid atlas face
            for i in start_idx..end_idx {
                unsafe {
                    *a_ptr.add(i) = source_uvs[i];
                    *l_ptr.add(i) = source_uvs[i];
                }
            }
            return FaceOut {
                chunk_id: 0,
                texture_id: 0,
                uv_mode: 0,
                is_overlay,
                success: false,
            };
        }

        // Standard material path
        if let Some((_, sprite_loc)) = MaterialResolver::resolve(mat_name, aliases, address_map) {
            let uv_mode = if is_overlay { 3 } else { 0 };
            for i in start_idx..end_idx {
                let [u_in, v_in] = source_uvs[i];
                let target_atlas_uv = remap_local_to_atlas(u_in, v_in, sprite_loc);
                unsafe {
                    *a_ptr.add(i) = target_atlas_uv;
                    *l_ptr.add(i) = [u_in, v_in];
                }
            }
            FaceOut {
                chunk_id: sprite_loc.chunk_id,
                texture_id: sprite_loc.texture_id,
                uv_mode,
                is_overlay,
                success: true,
            }
        } else {
            for i in start_idx..end_idx {
                unsafe {
                    *a_ptr.add(i) = source_uvs[i];
                    *l_ptr.add(i) = source_uvs[i];
                }
            }
            FaceOut {
                chunk_id: 0,
                texture_id: 0,
                uv_mode: 0,
                is_overlay,
                success: false,
            }
        }
    };

    #[cfg(feature = "parallel")]
    let face_results: Vec<FaceOut> = (0..face_count).into_par_iter().map(map_face).collect();

    #[cfg(not(feature = "parallel"))]
    let face_results: Vec<FaceOut> = (0..face_count).into_iter().map(map_face).collect();

    let mut face_chunk_ids = vec![0u16; face_count];
    let mut face_texture_ids = vec![0u32; face_count];
    let mut face_uv_modes = vec![0u8; face_count];
    let mut face_is_overlay = vec![false; face_count];
    let mut mapped_faces = 0usize;

    for (i, out) in face_results.into_iter().enumerate() {
        face_chunk_ids[i] = out.chunk_id;
        face_texture_ids[i] = out.texture_id;
        face_uv_modes[i] = out.uv_mode;
        face_is_overlay[i] = out.is_overlay;
        if out.success {
            mapped_faces += 1;
        }
    }

    crate::types::MeshMultiUvRemapResult {
        face_count,
        loop_count: total_uv_len,
        atlas_uvs,
        local_uvs,
        face_chunk_ids,
        face_texture_ids,
        face_uv_modes,
        face_is_overlay,
        unmapped_faces: face_count.saturating_sub(mapped_faces),
    }
}
