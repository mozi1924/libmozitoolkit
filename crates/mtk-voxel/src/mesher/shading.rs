//! # Meshing Shading, Material, CTM, and Biome Resolver
//!
//! Encapsulates material resolution, CTM solver querying, Atlas UV lookup,
//! and Biome color sampling for voxel meshing, decoupling material logic
//! from voxel traversal.

use std::sync::Arc;
use glam::{IVec3, Vec2};
use mtk_core::direction::Direction;
use mtk_material::MaterialResolver;
use mtk_model::baked::{BakedFace, BakedModel};

use crate::biome::SmoothedBiomeColumn;
use crate::mesher::heuristic::get_unit_cube_texture_candidates;
use crate::mesher::tint::compute_face_tint;
use crate::storage::PaddedVoxelArray;
use crate::types::MesherConfig;

/// Pre-resolved shading information for a static face.
#[derive(Clone, Debug)]
pub struct PreResolvedFace {
    pub source_texture_key: String,
    pub override_uvs: Option<[Vec2; 4]>,
    pub mat_slot: u16,
    pub chunk_id: i32,
    pub tex_id: u32,
    pub tint_data: [f32; 4],
    pub tint_color: [f32; 4],
    pub colormap_uv: [f32; 3],
    pub emission: f32,
}

/// Pre-resolved model face bundling the original BakedFace geometry and pre-resolved shading.
#[derive(Clone, Debug)]
pub struct PreResolvedModelFace {
    pub face: BakedFace,
    pub pre: PreResolvedFace,
}

/// Pre-resolved meshing data for a palette entry (either complex BakedModel or simple UnitCube).
pub enum PaletteMeshingData {
    Model {
        culled_faces: [Vec<PreResolvedModelFace>; 6],
        unculled_faces: Vec<PreResolvedModelFace>,
    },
    UnitCube {
        faces: [PreResolvedFace; 6],
    },
}

/// Unified fully-resolved shading and material data for an emitted face.
#[derive(Debug, Clone)]
pub struct ResolvedFaceShading {
    pub texture_key: String,
    pub override_uvs: Option<[Vec2; 4]>,
    pub mat_slot: u16,
    pub chunk_id: i32,
    pub tex_id: u32,
    pub tint_data: [f32; 4],
    pub tint_color: [f32; 4],
    pub colormap_uv: [f32; 3],
    pub emission: f32,
}

/// Pre-resolves palette meshing data (Atlas UVs, material slots, tint) outside the meshing hot loop.
pub fn build_palette_meshing_data(
    padded: &PaddedVoxelArray,
    palette_models: &[Option<Arc<BakedModel>>],
    config: &MesherConfig,
) -> Vec<PaletteMeshingData> {
    padded
        .palette
        .iter()
        .zip(palette_models.iter())
        .map(|(state_str, model_opt)| {
            let clean_block = mtk_resource::extract_block_name(state_str);
            let emission = if let Some(ref baked) = model_opt {
                baked.emissive_level
            } else {
                mtk_model::baker::get_block_emissive_level(
                    &mtk_model::blockstate::BlockState::parse(state_str).unwrap_or_default(),
                )
            };

            if let Some(baked) = model_opt {
                let (c_buckets, u_bucket) = baked.get_face_buckets();

                let resolve_face = |face: &BakedFace| -> PreResolvedModelFace {
                    let (final_tex_key, override_uvs, mat_slot, chunk_id, tex_id) =
                        if let Some(ref atlas_uvs) = face.atlas_uvs {
                            (
                                face.texture.clone(),
                                Some(*atlas_uvs),
                                face.atlas_chunk_id.unwrap_or(0),
                                face.atlas_chunk_id.unwrap_or(0) as i32,
                                face.atlas_texture_id.unwrap_or(0),
                            )
                        } else if let Some(atlas) = &config.atlas_address_map {
                            let base_loc = if !face.texture.is_empty() {
                                mtk_resource::ResourceLocation::parse(&face.texture).ok()
                            } else {
                                None
                            };
                            let resolved = if let Some(ref loc) = base_loc {
                                MaterialResolver::resolve(
                                    &loc.as_string(),
                                    config.custom_aliases.as_deref(),
                                    atlas,
                                )
                                .or_else(|| atlas.lookup(loc).map(|sp| ((*loc).clone(), sp)))
                            } else {
                                None
                            }
                            .or_else(|| {
                                MaterialResolver::resolve(
                                    &face.texture,
                                    config.custom_aliases.as_deref(),
                                    atlas,
                                )
                            });

                            if let Some((res_loc, atlas_loc)) = resolved {
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
                                (
                                    res_loc.as_string(),
                                    Some(remapped),
                                    atlas_loc.chunk_id,
                                    atlas_loc.chunk_id as i32,
                                    atlas_loc.texture_id,
                                )
                            } else {
                                (face.texture.clone(), None, 0, 0, 0)
                            }
                        } else {
                            (face.texture.clone(), None, 0, 0, 0)
                        };

                    let (tint_data, tint_color, colormap_uv) = compute_face_tint(
                        &final_tex_key,
                        state_str,
                        face.tint_index,
                        config.biome_resolver.as_deref(),
                    );

                    PreResolvedModelFace {
                        face: face.clone(),
                        pre: PreResolvedFace {
                            source_texture_key: final_tex_key,
                            override_uvs,
                            mat_slot,
                            chunk_id,
                            tex_id,
                            tint_data,
                            tint_color,
                            colormap_uv,
                            emission,
                        },
                    }
                };

                let mut culled_faces: [Vec<PreResolvedModelFace>; 6] = Default::default();
                for dir in Direction::ALL {
                    let idx = dir.to_index();
                    culled_faces[idx] = c_buckets[idx].iter().map(&resolve_face).collect();
                }
                let unculled_faces: Vec<PreResolvedModelFace> =
                    u_bucket.iter().map(&resolve_face).collect();

                PaletteMeshingData::Model {
                    culled_faces,
                    unculled_faces,
                }
            } else {
                let clean_sub = clean_block
                    .strip_prefix("minecraft:")
                    .unwrap_or(clean_block);

                let mut faces: [Option<PreResolvedFace>; 6] = Default::default();
                for dir in Direction::ALL {
                    let (final_tex_key, override_uvs, mat_slot, chunk_id, tex_id) =
                        if let Some(atlas) = &config.atlas_address_map {
                            let candidates = get_unit_cube_texture_candidates(clean_sub, dir);
                            let mut resolved = None;
                            for cand in &candidates {
                                if let Some((res, sp)) = MaterialResolver::resolve(
                                    cand,
                                    config.custom_aliases.as_deref(),
                                    atlas,
                                ) {
                                    resolved = Some((res, sp));
                                    break;
                                }
                            }
                            if let Some((res_loc, atlas_loc)) = resolved {
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
                                (
                                    res_loc.as_string(),
                                    Some(remapped),
                                    atlas_loc.chunk_id,
                                    atlas_loc.chunk_id as i32,
                                    atlas_loc.texture_id,
                                )
                            } else {
                                (format!("minecraft:block/{}", clean_sub), None, 0, 0, 0)
                            }
                        } else {
                            (format!("minecraft:block/{}", clean_sub), None, 0, 0, 0)
                        };

                    let tint_idx = mtk_material::get_unit_cube_tint_index(clean_sub, dir);
                    let (tint_data, tint_color, colormap_uv) = compute_face_tint(
                        &final_tex_key,
                        state_str,
                        tint_idx,
                        config.biome_resolver.as_deref(),
                    );

                    faces[dir.to_index()] = Some(PreResolvedFace {
                        source_texture_key: final_tex_key,
                        override_uvs,
                        mat_slot,
                        chunk_id,
                        tex_id,
                        tint_data,
                        tint_color,
                        colormap_uv,
                        emission,
                    });
                }

                PaletteMeshingData::UnitCube {
                    faces: [
                        faces[0].take().unwrap(),
                        faces[1].take().unwrap(),
                        faces[2].take().unwrap(),
                        faces[3].take().unwrap(),
                        faces[4].take().unwrap(),
                        faces[5].take().unwrap(),
                    ],
                }
            }
        })
        .collect()
}

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
        if let Some(resolved_loc) = solver.resolve_face(
            state_str,
            face_dir,
            block_pos,
            base_loc.as_ref(),
            None,
            get_neighbor_state,
        ) {
            if let Some(atlas) = &config.atlas_address_map {
                if let Some(atlas_loc) = atlas.lookup(&resolved_loc) {
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
                    };
                }
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
        let base_loc = mtk_resource::ResourceLocation::new(
            "minecraft",
            format!("block/{}", clean_block),
        );
        if let Some(resolved_loc) = solver.resolve_face(
            state_str,
            dir,
            block_pos,
            Some(&base_loc),
            None,
            get_neighbor_state,
        ) {
            if let Some(atlas) = &config.atlas_address_map {
                if let Some(atlas_loc) = atlas.lookup(&resolved_loc) {
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
                    };
                }
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
    }
}

/// Samples per-column biome tint colors and colormap UVs based on face tint configuration.
pub fn sample_biome_tint(
    biome_data: Option<&[SmoothedBiomeColumn]>,
    col_idx: usize,
    tint_data: [f32; 4],
    default_tint_color: [f32; 4],
    default_colormap_uv: [f32; 3],
) -> ([f32; 4], [f32; 3]) {
    if let Some(biome_cols) = biome_data {
        if col_idx < biome_cols.len() && tint_data[2] > 0.0 {
            let col = &biome_cols[col_idx];
            let uv = [col.colormap_uv[0], col.colormap_uv[1], 0.0];
            let c = match tint_data[3] as u8 {
                mtk_material::TINT_TYPE_GRASS => col.grass_color,
                mtk_material::TINT_TYPE_FOLIAGE => col.foliage_color,
                mtk_material::TINT_TYPE_DRY_FOLIAGE => col.dry_foliage_color,
                mtk_material::TINT_TYPE_WATER => col.water_color,
                mtk_material::TINT_TYPE_HARDCODED => default_tint_color,
                _ => default_tint_color,
            };
            (c, uv)
        } else {
            (default_tint_color, default_colormap_uv)
        }
    } else {
        (default_tint_color, default_colormap_uv)
    }
}
