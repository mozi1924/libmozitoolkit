use std::collections::HashMap;

use mtk_texture::{AtlasAddressMap, AtlasSpriteLocation};
#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::remap::{
    detect_face_uv_rotation, normalize_face_uv_for_atlas_tiling, remap_local_to_atlas,
    straighten_face_uv,
};
use crate::resolver::{
    decode_grid_atlas_uv, extract_animation_metrics, resolve_routed_sprite, MaterialResolver,
};
use crate::types::{
    GridAtlasSpec, MeshMultiUvRemapResult, MeshPalettedUvRemapResult, MeshRemapResult,
};

#[inline]
fn is_contiguous_partition(ranges: &[(u32, u32)], total_len: usize) -> bool {
    let mut expected = 0usize;
    for &(start, count) in ranges {
        if start as usize != expected {
            return false;
        }
        expected = match expected.checked_add(count as usize) {
            Some(v) => v,
            None => return false,
        };
    }
    expected == total_len
}

fn remap_single_face_uvs(
    face_uvs: &mut [[f32; 2]],
    mat_name: &str,
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
) -> (u16, u32, bool) {
    let count = face_uvs.len();
    if count == 0 {
        return (0, 0, false);
    }

    if let Some(spec) = grid_atlas_spec.filter(|s| s.matches_atlas_name(mat_name)) {
        let mut sum_u = 0.0f32;
        let mut sum_v = 0.0f32;
        for &[u, v] in face_uvs.iter() {
            sum_u += u;
            sum_v += v;
        }
        let avg_u = sum_u / (count as f32);
        let avg_v = sum_v / (count as f32);

        if let Some((_, sprite_loc, _)) =
            MaterialResolver::resolve_grid_atlas_face(avg_u, avg_v, spec, aliases, address_map)
        {
            for uv in face_uvs.iter_mut() {
                let [u_in, v_in] = *uv;
                let (_, local_uv) = decode_grid_atlas_uv(u_in, v_in, spec);
                *uv = remap_local_to_atlas(local_uv[0], local_uv[1], sprite_loc);
            }
            return (sprite_loc.chunk_id, sprite_loc.texture_id, true);
        }
        return (0, 0, false);
    }

    if let Some((_, sprite_loc)) = MaterialResolver::resolve(mat_name, aliases, address_map) {
        let mut local_face_uvs = face_uvs.to_vec();
        let angle = detect_face_uv_rotation(&local_face_uvs, 1e-3);
        if angle.abs() > 1e-4 {
            straighten_face_uv(&mut local_face_uvs, angle);
        }
        let _ = normalize_face_uv_for_atlas_tiling(&mut local_face_uvs, 1e-6);

        for (uv, &[u_norm, v_norm]) in face_uvs.iter_mut().zip(&local_face_uvs) {
            *uv = remap_local_to_atlas(u_norm, v_norm, sprite_loc);
        }
        (sprite_loc.chunk_id, sprite_loc.texture_id, true)
    } else {
        (0, 0, false)
    }
}

/// Parallel batch remapper for entire mesh UV layers and face material assignments.
///
/// This legacy entry point always prefers the static Frame-0 baseline. Use
/// [`remap_mesh_multi_uvs_parallel_with_options`] when animated-atlas routing is
/// required.
pub fn remap_mesh_uvs_parallel(
    uvs: &mut [[f32; 2]],
    face_materials: &[String],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
) -> MeshRemapResult {
    let face_count = face_materials.len();
    let total_uv_len = uvs.len();
    if face_count != face_loop_ranges.len() {
        return MeshRemapResult {
            face_count,
            loop_count: total_uv_len,
            face_chunk_ids: vec![0; face_count],
            face_texture_ids: vec![0; face_count],
            unmapped_faces: face_count,
        };
    }

    if is_contiguous_partition(face_loop_ranges, total_uv_len) {
        let mut remaining = uvs;
        let mut face_uv_slices = Vec::with_capacity(face_count);
        for &(_, count) in face_loop_ranges {
            let (curr, rest) = remaining.split_at_mut(count as usize);
            face_uv_slices.push(curr);
            remaining = rest;
        }

        #[cfg(feature = "parallel")]
        let face_results: Vec<(u16, u32, bool)> = face_materials
            .par_iter()
            .zip(face_uv_slices.into_par_iter())
            .map(|(mat, face_uvs)| {
                remap_single_face_uvs(face_uvs, mat, address_map, aliases, grid_atlas_spec)
            })
            .collect();

        #[cfg(not(feature = "parallel"))]
        let face_results: Vec<(u16, u32, bool)> = face_materials
            .iter()
            .zip(face_uv_slices.into_iter())
            .map(|(mat, face_uvs)| {
                remap_single_face_uvs(face_uvs, mat, address_map, aliases, grid_atlas_spec)
            })
            .collect();

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
    } else {
        // Safe fallback for non-contiguous or sparse ranges
        let mut face_chunk_ids = vec![0u16; face_count];
        let mut face_texture_ids = vec![0u32; face_count];
        let mut mapped_faces = 0usize;

        for (face_idx, &(start, count)) in face_loop_ranges.iter().enumerate() {
            let start_idx = start as usize;
            let end_idx = start_idx + (count as usize);
            if start_idx >= total_uv_len || end_idx > total_uv_len || count == 0 {
                continue;
            }
            let mat_name = &face_materials[face_idx];
            let (chunk_id, tex_id, success) = remap_single_face_uvs(
                &mut uvs[start_idx..end_idx],
                mat_name,
                address_map,
                aliases,
                grid_atlas_spec,
            );
            face_chunk_ids[face_idx] = chunk_id;
            face_texture_ids[face_idx] = tex_id;
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
}

struct FaceOut {
    chunk_id: u16,
    texture_id: u32,
    uv_transform: [f32; 4],
    uv_rotation: f32,
    uv_mode: u8,
    is_overlay: bool,
    is_animated: bool,
    anim_timing: [f32; 3],
    anim_frame_size: [f32; 3],
    success: bool,
}

fn default_face_out(is_overlay: bool) -> FaceOut {
    FaceOut {
        chunk_id: 0,
        texture_id: 0,
        uv_transform: [1.0, 1.0, 0.0, 0.0],
        uv_rotation: 0.0,
        uv_mode: 0,
        is_overlay,
        is_animated: false,
        anim_timing: [1.0, 1.0, 0.0],
        anim_frame_size: [16.0, 16.0, 0.0],
        success: false,
    }
}

fn route_fields(sprite: &AtlasSpriteLocation) -> (bool, [f32; 3], [f32; 3]) {
    let (timing, frame_size) = extract_animation_metrics(Some(sprite));
    (sprite.is_animated, timing, frame_size)
}

#[allow(clippy::too_many_arguments)]
fn remap_single_face_multi_uvs(
    mat_name: &str,
    src_uvs: &[[f32; 2]],
    out_atlas_uvs: &mut [[f32; 2]],
    out_local_uvs: &mut [[f32; 2]],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
    enable_animated_textures: bool,
) -> FaceOut {
    let count = src_uvs.len();
    if count == 0 {
        return default_face_out(false);
    }

    let is_overlay = mat_name.contains("overlay") || mat_name.contains("grass_side_overlay");

    if let Some(spec) = grid_atlas_spec.filter(|s| s.matches_atlas_name(mat_name)) {
        let mut sum_u = 0.0f32;
        let mut sum_v = 0.0f32;
        for &[u_in, v_in] in src_uvs {
            sum_u += u_in;
            sum_v += v_in;
        }
        let avg_u = sum_u / (count as f32);
        let avg_v = sum_v / (count as f32);

        if let Some((res_loc, mut sprite_loc, _)) =
            MaterialResolver::resolve_grid_atlas_face(avg_u, avg_v, spec, aliases, address_map)
        {
            if enable_animated_textures {
                if let Some(anim) = address_map.lookup_animated(&res_loc) {
                    sprite_loc = anim;
                }
            }
            let uv_mode = if is_overlay { 3 } else { 0 };
            for (i, &[u_in, v_in]) in src_uvs.iter().enumerate() {
                let (_, local_uv) = decode_grid_atlas_uv(u_in, v_in, spec);
                let target_atlas_uv = remap_local_to_atlas(local_uv[0], local_uv[1], sprite_loc);
                out_atlas_uvs[i] = target_atlas_uv;
                out_local_uvs[i] = local_uv;
            }
            let (is_animated, anim_timing, anim_frame_size) = route_fields(sprite_loc);
            return FaceOut {
                chunk_id: sprite_loc.chunk_id,
                texture_id: sprite_loc.texture_id,
                uv_transform: [1.0, 1.0, 0.0, 0.0],
                uv_rotation: 0.0,
                uv_mode,
                is_overlay,
                is_animated,
                anim_timing,
                anim_frame_size,
                success: true,
            };
        }

        // Unresolved grid atlas face
        out_atlas_uvs.copy_from_slice(src_uvs);
        out_local_uvs.copy_from_slice(src_uvs);
        return default_face_out(is_overlay);
    }

    // Standard material path
    if let Some((_, sprite_loc)) =
        resolve_routed_sprite(mat_name, aliases, address_map, enable_animated_textures)
    {
        let uv_mode = if is_overlay { 3 } else { 0 };

        // 1. Copy face loop UVs
        let mut face_uvs = src_uvs.to_vec();

        // 2. Detect and straighten rotated UVs (e.g. jmc2obj flowing liquid at 45 deg)
        let angle = detect_face_uv_rotation(&face_uvs, 1e-3);
        if angle.abs() > 1e-4 {
            straighten_face_uv(&mut face_uvs, angle);
        }

        // 3. Normalize tiled UVs to [0, 1] if required and extract affine transform
        let (uv_trans, is_tiled) = normalize_face_uv_for_atlas_tiling(&mut face_uvs, 1e-6);

        // 4. Map normalized UVs to Atlas and write outputs
        for i in 0..count {
            let [u_norm, v_norm] = face_uvs[i];
            let target_atlas_uv = remap_local_to_atlas(u_norm, v_norm, sprite_loc);
            out_atlas_uvs[i] = target_atlas_uv;
            out_local_uvs[i] = if is_tiled || angle.abs() > 1e-4 {
                [u_norm, v_norm]
            } else {
                src_uvs[i]
            };
        }

        let (is_animated, anim_timing, anim_frame_size) = route_fields(sprite_loc);
        FaceOut {
            chunk_id: sprite_loc.chunk_id,
            texture_id: sprite_loc.texture_id,
            uv_transform: uv_trans,
            uv_rotation: angle,
            uv_mode,
            is_overlay,
            is_animated,
            anim_timing,
            anim_frame_size,
            success: true,
        }
    } else {
        out_atlas_uvs.copy_from_slice(src_uvs);
        out_local_uvs.copy_from_slice(src_uvs);
        default_face_out(is_overlay)
    }
}

/// Parallel batch remapper producing both Atlas UVs (chunk-mapped) and Local UVs ([0..1] normalized),
/// along with per-face UV mode routing flags and overlay detection.
///
/// Legacy entry point preserving the static Frame-0 policy. Prefer
/// [`remap_mesh_multi_uvs_parallel_with_options`] when the host exposes an
/// animated-texture preference.
pub fn remap_mesh_multi_uvs_parallel(
    source_uvs: &[[f32; 2]],
    face_materials: &[String],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
) -> MeshMultiUvRemapResult {
    remap_mesh_multi_uvs_parallel_with_options(
        source_uvs,
        face_materials,
        face_loop_ranges,
        address_map,
        aliases,
        grid_atlas_spec,
        false,
    )
}

/// Material name source for per-face remapping: direct strings or a palette + indices.
#[derive(Clone, Copy)]
enum FaceMaterialSource<'a> {
    Strings(&'a [String]),
    Palette {
        palette: &'a [String],
        indices: &'a [u32],
    },
}

impl<'a> FaceMaterialSource<'a> {
    fn len(&self) -> usize {
        match self {
            Self::Strings(values) => values.len(),
            Self::Palette { indices, .. } => indices.len(),
        }
    }

    fn get(&self, index: usize) -> &'a str {
        match self {
            Self::Strings(values) => values.get(index).map(|s| s.as_str()).unwrap_or(""),
            Self::Palette { palette, indices } => indices
                .get(index)
                .and_then(|idx| palette.get(*idx as usize))
                .map(|s| s.as_str())
                .unwrap_or(""),
        }
    }
}

/// Animation-aware variant of [`remap_mesh_multi_uvs_parallel`].
///
/// When `enable_animated_textures` is true, multi-frame sprites are routed to
/// their dedicated animated atlas chunk and per-face animation metrics are
/// returned. When false, the static Frame-0 chunk remains the only target.
#[allow(clippy::too_many_arguments)]
pub fn remap_mesh_multi_uvs_parallel_with_options(
    source_uvs: &[[f32; 2]],
    face_materials: &[String],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
    enable_animated_textures: bool,
) -> MeshMultiUvRemapResult {
    remap_mesh_multi_uvs_impl(
        source_uvs,
        FaceMaterialSource::Strings(face_materials),
        face_loop_ranges,
        address_map,
        aliases,
        grid_atlas_spec,
        enable_animated_textures,
    )
}

/// Palette-native variant of [`remap_mesh_multi_uvs_parallel_with_options`].
///
/// Avoids constructing a `Vec<String>` with one entry per face. Also resolves the
/// per-face source-texture keys into a compact palette + u32 index array, with an
/// optional legacy provenance palette used as fallback for unmapped faces.
#[allow(clippy::too_many_arguments)]
pub fn remap_mesh_multi_uvs_paletted_with_options(
    source_uvs: &[[f32; 2]],
    face_material_palette: &[String],
    face_material_indices: &[u32],
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
    existing_key_palette: Option<&[String]>,
    existing_key_indices: Option<&[u32]>,
    enable_animated_textures: bool,
) -> MeshPalettedUvRemapResult {
    let materials = FaceMaterialSource::Palette {
        palette: face_material_palette,
        indices: face_material_indices,
    };
    let mesh = remap_mesh_multi_uvs_impl(
        source_uvs,
        materials,
        face_loop_ranges,
        address_map,
        aliases,
        grid_atlas_spec,
        enable_animated_textures,
    );
    let (source_key_palette, face_source_key_indices) = build_source_key_palette(
        &mesh.face_texture_ids,
        address_map,
        existing_key_palette,
        existing_key_indices,
    );
    MeshPalettedUvRemapResult {
        mesh,
        source_key_palette,
        face_source_key_indices,
    }
}

fn intern_source_key(
    key: &str,
    palette: &mut Vec<String>,
    lookup: &mut HashMap<String, u32>,
) -> u32 {
    if let Some(idx) = lookup.get(key) {
        return *idx;
    }
    let idx = palette.len() as u32;
    palette.push(key.to_string());
    lookup.insert(key.to_string(), idx);
    idx
}

fn build_source_key_palette(
    face_texture_ids: &[u32],
    address_map: &AtlasAddressMap,
    existing_key_palette: Option<&[String]>,
    existing_key_indices: Option<&[u32]>,
) -> (Vec<String>, Vec<u32>) {
    let mut owned_id_to_key: HashMap<u32, String> = HashMap::new();
    let mut register = |texture_id: u32, key: &str| {
        let key_string = key.to_string();
        match owned_id_to_key.get(&texture_id) {
            None => {
                owned_id_to_key.insert(texture_id, key_string);
            }
            Some(current) => {
                let current_is_overlay = current.contains("overlay");
                let new_is_overlay = key_string.contains("overlay");
                if (current_is_overlay && !new_is_overlay)
                    || (current_is_overlay == new_is_overlay && key_string.len() < current.len())
                {
                    owned_id_to_key.insert(texture_id, key_string);
                }
            }
        }
    };
    for (key, sprite) in &address_map.sprites {
        register(sprite.texture_id, &key.as_string());
    }
    for (key, sprite) in &address_map.anim_sprites {
        register(sprite.texture_id, &key.as_string());
    }

    let mut palette: Vec<String> = Vec::new();
    let mut lookup: HashMap<String, u32> = HashMap::new();
    let mut indices: Vec<u32> = Vec::with_capacity(face_texture_ids.len());

    for (i, texture_id) in face_texture_ids.iter().enumerate() {
        let atlas_key = owned_id_to_key.get(texture_id).map(|s| s.as_str());
        let legacy_key = existing_key_indices
            .and_then(|idx| idx.get(i))
            .and_then(|idx| existing_key_palette.and_then(|p| p.get(*idx as usize)))
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty() && *s != "mozi:fallback");

        let key = atlas_key.or(legacy_key).unwrap_or("mozi:fallback");
        indices.push(intern_source_key(key, &mut palette, &mut lookup));
    }

    (palette, indices)
}

#[allow(clippy::too_many_arguments)]
fn remap_mesh_multi_uvs_impl(
    source_uvs: &[[f32; 2]],
    materials: FaceMaterialSource<'_>,
    face_loop_ranges: &[(u32, u32)],
    address_map: &AtlasAddressMap,
    aliases: Option<&HashMap<String, Vec<String>>>,
    grid_atlas_spec: Option<&GridAtlasSpec>,
    enable_animated_textures: bool,
) -> MeshMultiUvRemapResult {
    let face_count = materials.len();
    let total_uv_len = source_uvs.len();

    if face_count != face_loop_ranges.len() {
        return MeshMultiUvRemapResult {
            face_count,
            loop_count: total_uv_len,
            atlas_uvs: source_uvs.to_vec(),
            local_uvs: source_uvs.to_vec(),
            face_chunk_ids: vec![0; face_count],
            face_texture_ids: vec![0; face_count],
            face_uv_transforms: vec![[1.0, 1.0, 0.0, 0.0]; face_count],
            face_uv_rotations: vec![0.0; face_count],
            face_uv_modes: vec![0; face_count],
            face_is_overlay: vec![false; face_count],
            face_is_animated: vec![false; face_count],
            face_anim_timing: vec![[1.0, 1.0, 0.0]; face_count],
            face_anim_frame_size: vec![[16.0, 16.0, 0.0]; face_count],
            unmapped_faces: face_count,
        };
    }

    let mut atlas_uvs = vec![[0.0f32, 0.0f32]; total_uv_len];
    let mut local_uvs = vec![[0.0f32, 0.0f32]; total_uv_len];

    let face_results: Vec<FaceOut> = if is_contiguous_partition(face_loop_ranges, total_uv_len) {
        let mut src_chunks = Vec::with_capacity(face_count);
        let mut curr_offset = 0;
        for &(_, count) in face_loop_ranges {
            let next_offset = curr_offset + count as usize;
            src_chunks.push(&source_uvs[curr_offset..next_offset]);
            curr_offset = next_offset;
        }

        let mut rem_atlas = atlas_uvs.as_mut_slice();
        let mut rem_local = local_uvs.as_mut_slice();
        let mut atlas_chunks = Vec::with_capacity(face_count);
        let mut local_chunks = Vec::with_capacity(face_count);
        for &(_, count) in face_loop_ranges {
            let (a_curr, a_rest) = rem_atlas.split_at_mut(count as usize);
            atlas_chunks.push(a_curr);
            rem_atlas = a_rest;

            let (l_curr, l_rest) = rem_local.split_at_mut(count as usize);
            local_chunks.push(l_curr);
            rem_local = l_rest;
        }

        #[cfg(feature = "parallel")]
        let res: Vec<FaceOut> = (0..face_count)
            .into_par_iter()
            .zip(src_chunks.into_par_iter())
            .zip(atlas_chunks.into_par_iter())
            .zip(local_chunks.into_par_iter())
            .map(|(((i, src), atlas_dst), local_dst)| {
                remap_single_face_multi_uvs(
                    materials.get(i),
                    src,
                    atlas_dst,
                    local_dst,
                    address_map,
                    aliases,
                    grid_atlas_spec,
                    enable_animated_textures,
                )
            })
            .collect();

        #[cfg(not(feature = "parallel"))]
        let res: Vec<FaceOut> = (0..face_count)
            .into_iter()
            .zip(src_chunks.into_iter())
            .zip(atlas_chunks.into_iter())
            .zip(local_chunks.into_iter())
            .map(|(((i, src), atlas_dst), local_dst)| {
                remap_single_face_multi_uvs(
                    materials.get(i),
                    src,
                    atlas_dst,
                    local_dst,
                    address_map,
                    aliases,
                    grid_atlas_spec,
                    enable_animated_textures,
                )
            })
            .collect();

        res
    } else {
        // Safe fallback for non-contiguous or sparse ranges
        atlas_uvs.copy_from_slice(source_uvs);
        local_uvs.copy_from_slice(source_uvs);
        let mut results = Vec::with_capacity(face_count);
        for (face_idx, &(start, count)) in face_loop_ranges.iter().enumerate() {
            let start_idx = start as usize;
            let end_idx = start_idx + (count as usize);
            if start_idx >= total_uv_len || end_idx > total_uv_len || count == 0 {
                results.push(default_face_out(false));
                continue;
            }
            let mat_name = materials.get(face_idx);
            let src = &source_uvs[start_idx..end_idx];
            let mut a_tmp = vec![[0.0, 0.0]; count as usize];
            let mut l_tmp = vec![[0.0, 0.0]; count as usize];
            let out = remap_single_face_multi_uvs(
                mat_name,
                src,
                &mut a_tmp,
                &mut l_tmp,
                address_map,
                aliases,
                grid_atlas_spec,
                enable_animated_textures,
            );
            atlas_uvs[start_idx..end_idx].copy_from_slice(&a_tmp);
            local_uvs[start_idx..end_idx].copy_from_slice(&l_tmp);
            results.push(out);
        }
        results
    };

    let mut face_chunk_ids = vec![0u16; face_count];
    let mut face_texture_ids = vec![0u32; face_count];
    let mut face_uv_transforms = vec![[1.0f32, 1.0f32, 0.0f32, 0.0f32]; face_count];
    let mut face_uv_rotations = vec![0.0f32; face_count];
    let mut face_uv_modes = vec![0u8; face_count];
    let mut face_is_overlay = vec![false; face_count];
    let mut face_is_animated = vec![false; face_count];
    let mut face_anim_timing = vec![[1.0f32, 1.0f32, 0.0f32]; face_count];
    let mut face_anim_frame_size = vec![[16.0f32, 16.0f32, 0.0f32]; face_count];
    let mut mapped_faces = 0usize;

    for (i, out) in face_results.into_iter().enumerate() {
        face_chunk_ids[i] = out.chunk_id;
        face_texture_ids[i] = out.texture_id;
        face_uv_transforms[i] = out.uv_transform;
        face_uv_rotations[i] = out.uv_rotation;
        face_uv_modes[i] = out.uv_mode;
        face_is_overlay[i] = out.is_overlay;
        face_is_animated[i] = out.is_animated;
        face_anim_timing[i] = out.anim_timing;
        face_anim_frame_size[i] = out.anim_frame_size;
        if out.success {
            mapped_faces += 1;
        }
    }

    MeshMultiUvRemapResult {
        face_count,
        loop_count: total_uv_len,
        atlas_uvs,
        local_uvs,
        face_chunk_ids,
        face_texture_ids,
        face_uv_transforms,
        face_uv_rotations,
        face_uv_modes,
        face_is_overlay,
        face_is_animated,
        face_anim_timing,
        face_anim_frame_size,
        unmapped_faces: face_count.saturating_sub(mapped_faces),
    }
}
