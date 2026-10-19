use glam::{IVec3, Vec2};
use mtk_core::direction::Direction;

use crate::biome::SmoothedBiomeColumn;
use crate::mesher::tint::compute_face_tint;
use crate::types::MesherConfig;

use super::anim::{extract_anim_metrics, lookup_animated_from_cache};
use super::types::{PreResolvedFace, PreResolvedModelFace, ResolvedFaceShading};

/// Dynamically resolves CTM overrides for a BakedModel face, falling back to pre-resolved shading.
pub fn resolve_model_face_shading<'a, F>(
    mf: &PreResolvedModelFace,
    state_str: &str,
    block_pos: IVec3,
    config: &MesherConfig,
    get_neighbor_state: F,
) -> ResolvedFaceShading
where
    F: Fn(IVec3) -> Option<&'a str>,
{
    let face = &mf.face;
    let pre_face = &mf.pre;
    let face_dir = face.direction;

    if let Some(solver) = &config.ctm_solver {
        let base_loc = if !face.texture.is_empty() {
            mtk_resource::ResourceLocation::parse(&face.texture).ok()
        } else {
            None
        };
        let tangents =
            mtk_core::derive_quad_face_tangents(&face.vertices, &face.uvs, face_dir, true);
        if let Some(resolved_loc) = solver.resolve_face_with_tangents(
            state_str,
            face_dir,
            tangents,
            block_pos,
            base_loc.as_ref(),
            None,
            get_neighbor_state,
        ) {
            if let Some(atlas) = &config.atlas_address_map {
                if let Some(mut atlas_loc) = atlas.lookup(&resolved_loc) {
                    if config.enable_animated_textures {
                        if let Some(anim_loc) = lookup_animated_from_cache(
                            &resolved_loc.as_string(),
                            config.custom_aliases.as_deref(),
                            atlas,
                        ) {
                            atlas_loc = anim_loc;
                        }
                    } else if atlas_loc.is_animated {
                        if let Some(static_loc) = atlas.lookup_static(&resolved_loc) {
                            atlas_loc = static_loc;
                        }
                    }
                    let u_min = atlas_loc.frame_0_uv_bounds[0];
                    let v_min = atlas_loc.frame_0_uv_bounds[1];
                    let u_span = atlas_loc.frame_0_uv_bounds[2] - u_min;
                    let v_span = atlas_loc.frame_0_uv_bounds[3] - v_min;
                    let remapped = [
                        Vec2::new(
                            u_min + face.uvs[0].x * u_span,
                            v_min + (1.0 - face.uvs[0].y) * v_span,
                        ),
                        Vec2::new(
                            u_min + face.uvs[1].x * u_span,
                            v_min + (1.0 - face.uvs[1].y) * v_span,
                        ),
                        Vec2::new(
                            u_min + face.uvs[2].x * u_span,
                            v_min + (1.0 - face.uvs[2].y) * v_span,
                        ),
                        Vec2::new(
                            u_min + face.uvs[3].x * u_span,
                            v_min + (1.0 - face.uvs[3].y) * v_span,
                        ),
                    ];
                    let ctm_key = resolved_loc.as_string();
                    let (td, tc, c_uv) = compute_face_tint(
                        &ctm_key,
                        state_str,
                        face.tint_index,
                        config.biome_resolver.as_deref(),
                    );
                    let (timing, fsize) = extract_anim_metrics(Some(atlas_loc));
                    let material_props = mtk_material::get_material_props("", Some(&ctm_key));
                    return ResolvedFaceShading {
                        texture_key: ctm_key,
                        override_uvs: Some(remapped),
                        mat_slot: atlas_loc.chunk_id,
                        chunk_id: atlas_loc.chunk_id as i32,
                        tex_id: atlas_loc.texture_id,
                        tint_data: td,
                        tint_color: tc,
                        colormap_uv: c_uv,
                        emission: pre_face.emission,
                        material_props,
                        anim_timing: timing,
                        anim_frame_size: fsize,
                    };
                }
            } else {
                let ctm_key = resolved_loc.as_string();
                let (td, tc, c_uv) = compute_face_tint(
                    &ctm_key,
                    state_str,
                    face.tint_index,
                    config.biome_resolver.as_deref(),
                );
                let material_props = mtk_material::get_material_props("", Some(&ctm_key));
                return ResolvedFaceShading {
                    texture_key: ctm_key,
                    override_uvs: None,
                    mat_slot: pre_face.mat_slot,
                    chunk_id: 0,
                    tex_id: 0,
                    tint_data: td,
                    tint_color: tc,
                    colormap_uv: c_uv,
                    emission: pre_face.emission,
                    material_props,
                    anim_timing: pre_face.anim_timing,
                    anim_frame_size: pre_face.anim_frame_size,
                };
            }
        }
    }

    ResolvedFaceShading {
        texture_key: pre_face.source_texture_key.clone(),
        override_uvs: pre_face.override_uvs,
        mat_slot: pre_face.mat_slot,
        chunk_id: pre_face.chunk_id,
        tex_id: pre_face.tex_id,
        tint_data: pre_face.tint_data,
        tint_color: pre_face.tint_color,
        colormap_uv: pre_face.colormap_uv,
        emission: pre_face.emission,
        material_props: pre_face.material_props,
        anim_timing: pre_face.anim_timing,
        anim_frame_size: pre_face.anim_frame_size,
    }
}

/// Dynamically resolves CTM overrides for a unit cube face, falling back to pre-resolved shading.
pub fn resolve_unit_cube_face_shading<'a, F>(
    pre_face: &PreResolvedFace,
    dir: Direction,
    state_str: &str,
    block_pos: IVec3,
    config: &MesherConfig,
    get_neighbor_state: F,
) -> ResolvedFaceShading
where
    F: Fn(IVec3) -> Option<&'a str>,
{
    if let Some(solver) = &config.ctm_solver {
        let clean_block = mtk_resource::extract_block_name(state_str)
            .strip_prefix("minecraft:")
            .unwrap_or(mtk_resource::extract_block_name(state_str));
        let base_loc =
            mtk_resource::ResourceLocation::new("minecraft", format!("block/{}", clean_block));
        if let Some(resolved_loc) = solver.resolve_face(
            state_str,
            dir,
            block_pos,
            Some(&base_loc),
            None,
            get_neighbor_state,
        ) {
            if let Some(atlas) = &config.atlas_address_map {
                if let Some(mut atlas_loc) = atlas.lookup(&resolved_loc) {
                    if config.enable_animated_textures {
                        if let Some(anim_loc) = lookup_animated_from_cache(
                            &resolved_loc.as_string(),
                            config.custom_aliases.as_deref(),
                            atlas,
                        ) {
                            atlas_loc = anim_loc;
                        }
                    } else if atlas_loc.is_animated {
                        if let Some(static_loc) = atlas.lookup_static(&resolved_loc) {
                            atlas_loc = static_loc;
                        }
                    }
                    let u_min = atlas_loc.frame_0_uv_bounds[0];
                    let v_min = atlas_loc.frame_0_uv_bounds[1];
                    let u_max = atlas_loc.frame_0_uv_bounds[2];
                    let v_max = atlas_loc.frame_0_uv_bounds[3];
                    let remapped = [
                        Vec2::new(u_min, v_max),
                        Vec2::new(u_min, v_min),
                        Vec2::new(u_max, v_min),
                        Vec2::new(u_max, v_max),
                    ];
                    let ctm_key = resolved_loc.as_string();
                    let (td, tc, c_uv) = compute_face_tint(
                        &ctm_key,
                        state_str,
                        -1,
                        config.biome_resolver.as_deref(),
                    );
                    let (timing, fsize) = extract_anim_metrics(Some(atlas_loc));
                    let material_props = mtk_material::get_material_props("", Some(&ctm_key));
                    return ResolvedFaceShading {
                        texture_key: ctm_key,
                        override_uvs: Some(remapped),
                        mat_slot: atlas_loc.chunk_id,
                        chunk_id: atlas_loc.chunk_id as i32,
                        tex_id: atlas_loc.texture_id,
                        tint_data: td,
                        tint_color: tc,
                        colormap_uv: c_uv,
                        emission: pre_face.emission,
                        material_props,
                        anim_timing: timing,
                        anim_frame_size: fsize,
                    };
                }
            } else {
                let ctm_key = resolved_loc.as_string();
                let (td, tc, c_uv) =
                    compute_face_tint(&ctm_key, state_str, -1, config.biome_resolver.as_deref());
                let material_props = mtk_material::get_material_props("", Some(&ctm_key));
                return ResolvedFaceShading {
                    texture_key: ctm_key,
                    override_uvs: None,
                    mat_slot: pre_face.mat_slot,
                    chunk_id: 0,
                    tex_id: 0,
                    tint_data: td,
                    tint_color: tc,
                    colormap_uv: c_uv,
                    emission: pre_face.emission,
                    material_props,
                    anim_timing: pre_face.anim_timing,
                    anim_frame_size: pre_face.anim_frame_size,
                };
            }
        }
    }

    ResolvedFaceShading {
        texture_key: pre_face.source_texture_key.clone(),
        override_uvs: pre_face.override_uvs,
        mat_slot: pre_face.mat_slot,
        chunk_id: pre_face.chunk_id,
        tex_id: pre_face.tex_id,
        tint_data: pre_face.tint_data,
        tint_color: pre_face.tint_color,
        colormap_uv: pre_face.colormap_uv,
        emission: pre_face.emission,
        material_props: pre_face.material_props,
        anim_timing: pre_face.anim_timing,
        anim_frame_size: pre_face.anim_frame_size,
    }
}

/// Samples per-column/3D quart biome tint colors and colormap UVs based on face tint configuration.
pub fn sample_biome_tint(
    biome_data: Option<&[SmoothedBiomeColumn]>,
    lx: usize,
    ly: usize,
    lz: usize,
    mut tint_data: [f32; 4],
    default_tint_color: [f32; 4],
    default_colormap_uv: [f32; 3],
) -> ([f32; 4], [f32; 4], [f32; 3]) {
    if tint_data[2] <= 0.0 {
        return (tint_data, default_tint_color, default_colormap_uv);
    }
    if let Some(biome_cols) = biome_data {
        let col_idx = if biome_cols.len() == 1024 {
            let qy = (ly >> 2) & 3;
            (qy << 8) | ((lx & 15) << 4) | (lz & 15)
        } else {
            (lx & 15) * 16 + (lz & 15)
        };
        if col_idx < biome_cols.len() && tint_data[2] > 0.0 {
            let col = &biome_cols[col_idx];
            let uv = [col.colormap_uv[0], col.colormap_uv[1], 0.0];
            let tt = tint_data[3] as u8;
            let (c, has_custom) = match tt {
                mtk_material::TINT_TYPE_GRASS => (col.grass_color, col.has_custom_grass),
                mtk_material::TINT_TYPE_FOLIAGE => (col.foliage_color, col.has_custom_foliage),
                mtk_material::TINT_TYPE_DRY_FOLIAGE => {
                    (col.dry_foliage_color, col.has_custom_dry_foliage)
                }
                mtk_material::TINT_TYPE_WATER => (col.water_color, false),
                mtk_material::TINT_TYPE_HARDCODED => (default_tint_color, true),
                _ => (default_tint_color, false),
            };
            if has_custom {
                tint_data[3] = mtk_material::TINT_TYPE_HARDCODED as f32;
            }
            (tint_data, c, uv)
        } else {
            (tint_data, default_tint_color, default_colormap_uv)
        }
    } else {
        (tint_data, default_tint_color, default_colormap_uv)
    }
}
