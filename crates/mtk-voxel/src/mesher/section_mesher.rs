use std::sync::Arc;

use glam::{IVec3, Vec3};
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use mtk_cull::types::BlockCullMeta;
use mtk_cull::FaceCuller;
use mtk_material::MaterialResolver;
use mtk_model::baked::{BakedFace, BakedModel};
use mtk_model::baker::is_block_emissive;
use mtk_model::blockstate::BlockState;

use crate::ao::{ao_level_to_brightness, calculate_face_ao, should_flip_quad_diagonal};
use crate::fluid::emit_fluid_geometry;
use crate::storage::{padded_index, PaddedVoxelArray, SECTION_SIZE};
use crate::types::{MesherConfig, VoxelError};

use super::collector::FaceAttributesCollector;
use super::heuristic::get_unit_cube_texture_candidates;
use super::tint::compute_face_tint;

#[derive(Clone, Debug)]
pub(crate) struct PreResolvedFace {
    pub(crate) source_texture_key: String,
    pub(crate) override_uvs: Option<[glam::Vec2; 4]>,
    pub(crate) mat_slot: u16,
    pub(crate) chunk_id: i32,
    pub(crate) tex_id: u32,
    pub(crate) tint_data: [f32; 4],
    pub(crate) tint_color: [f32; 4],
    pub(crate) colormap_uv: [f32; 3],
}

#[derive(Clone, Debug)]
pub(crate) struct PreResolvedModelFace {
    pub(crate) face: BakedFace,
    pub(crate) pre: PreResolvedFace,
}

pub(crate) enum PaletteMeshingData {
    Model {
        culled_faces: [Vec<PreResolvedModelFace>; 6],
        unculled_faces: Vec<PreResolvedModelFace>,
    },
    UnitCube {
        faces: [PreResolvedFace; 6],
    },
}

/// High-performance mesh generator for individual and batch chunk sections.
pub struct SectionMesher;

impl SectionMesher {
    /// Meshes a single `PaddedVoxelArray` into a complete `MeshData` buffer.
    pub fn mesh_section<F>(
        padded: &PaddedVoxelArray,
        culler: &FaceCuller,
        mut get_baked_model: F,
        config: &MesherConfig,
    ) -> MeshData
    where
        F: FnMut(&str) -> Option<Arc<BakedModel>>,
    {
        if padded.is_empty {
            return MeshData::new();
        }

        let mut mesh = MeshData::with_capacity(1024, 1536, 512);
        let mut collector = FaceAttributesCollector::with_capacity(512);

        // Pre-resolve baked models FIRST
        let mut palette_models: Vec<Option<Arc<BakedModel>>> =
            Vec::with_capacity(padded.palette.len());
        for st in &padded.palette {
            palette_models.push(get_baked_model(st));
        }

        // Pre-resolve palette metadata using baked models (fast path: pre-baked cull_meta)
        let palette_metas: Vec<Arc<BlockCullMeta>> = padded
            .palette
            .iter()
            .zip(palette_models.iter())
            .map(|(st, model_opt)| {
                if let Some(model) = model_opt {
                    Arc::new(model.get_or_compute_cull_meta())
                } else {
                    culler.get_meta(st, None, None)
                }
            })
            .collect();

        // Pre-resolve palette emissive states (avoids repeated BlockState::parse in hot loop)
        let palette_emissive: Vec<bool> = padded
            .palette
            .iter()
            .zip(palette_models.iter())
            .map(|(st, model_opt)| {
                if let Some(model) = model_opt {
                    model.is_emissive
                } else {
                    BlockState::parse(st).as_ref().map(is_block_emissive).unwrap_or(false)
                }
            })
            .collect();

        // Pre-resolve palette meshing data (Atlas UVs, material slots, tint) outside hot loop
        let palette_meshing_data: Vec<PaletteMeshingData> = padded
            .palette
            .iter()
            .zip(palette_models.iter())
            .map(|(state_str, model_opt)| {
                let clean_block = mtk_resource::extract_block_name(state_str);
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
                                        glam::Vec2::new(
                                            u_min + face.uvs[0].x * u_span,
                                            v_min + (1.0 - face.uvs[0].y) * v_span,
                                        ),
                                        glam::Vec2::new(
                                            u_min + face.uvs[1].x * u_span,
                                            v_min + (1.0 - face.uvs[1].y) * v_span,
                                        ),
                                        glam::Vec2::new(
                                            u_min + face.uvs[2].x * u_span,
                                            v_min + (1.0 - face.uvs[2].y) * v_span,
                                        ),
                                        glam::Vec2::new(
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
                            clean_block,
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
                            },
                        }
                    };

                    let mut culled_faces: [Vec<PreResolvedModelFace>; 6] = Default::default();
                    for dir in Direction::ALL {
                        let idx = dir.to_index();
                        culled_faces[idx] = c_buckets[idx].iter().map(&resolve_face).collect();
                    }
                    let unculled_faces: Vec<PreResolvedModelFace> = u_bucket.iter().map(&resolve_face).collect();

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
                                        glam::Vec2::new(u_min, v_max),
                                        glam::Vec2::new(u_min, v_min),
                                        glam::Vec2::new(u_max, v_min),
                                        glam::Vec2::new(u_max, v_max),
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

                        let tint_idx = if (clean_sub == "grass_block" || clean_sub == "grass") && dir == Direction::Up {
                            0
                        } else {
                            -1
                        };
                        let (tint_data, tint_color, colormap_uv) = compute_face_tint(
                            &final_tex_key,
                            clean_block,
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
            .collect();

        let world_offset_x = (padded.coord.x * 16) as f32;
        let world_offset_y = (padded.coord.y * 16) as f32;
        let world_offset_z = (padded.coord.z * 16) as f32;

        let is_opaque_fn = |px: usize, py: usize, pz: usize| -> bool {
            let idx = padded.get_padded_idx(px, py, pz) as usize;
            if idx < palette_metas.len() {
                palette_metas[idx].is_opaque
            } else {
                false
            }
        };

        for lx in 0..SECTION_SIZE {
            let px = lx + 1;
            let wx = world_offset_x + lx as f32;

            for ly in 0..SECTION_SIZE {
                let py = ly + 1;
                let wy = world_offset_y + ly as f32;

                for lz in 0..SECTION_SIZE {
                    let pz = lz + 1;
                    let wz = world_offset_z + lz as f32;

                    let pal_idx = padded.get_padded_idx(px, py, pz) as usize;
                    if pal_idx >= palette_metas.len() {
                        continue;
                    }

                    let meta = &palette_metas[pal_idx];
                    if meta.is_air {
                        continue;
                    }

                    let state_str = &padded.palette[pal_idx];
                    let block_pos = IVec3::new(wx as i32, wy as i32, wz as i32);

                    // 1. Fluid Meshing (Water, Lava, or Waterlogged)
                    if config.mesh_fluids && (meta.is_fluid || meta.is_waterlogged) {
                        let get_state = |nx: i32, ny: i32, nz: i32| -> String {
                            let n_px = (nx - block_pos.x + px as i32) as usize;
                            let n_py = (ny - block_pos.y + py as i32) as usize;
                            let n_pz = (nz - block_pos.z + pz as i32) as usize;
                            if n_px < 18 && n_py < 18 && n_pz < 18 {
                                padded.get_padded_state(n_px, n_py, n_pz).to_string()
                            } else {
                                "minecraft:air".to_string()
                            }
                        };

                        emit_fluid_geometry(
                            &mut mesh,
                            block_pos.x,
                            block_pos.y,
                            block_pos.z,
                            wx,
                            wy,
                            wz,
                            state_str,
                            get_state,
                            culler,
                            config,
                            0,
                            Some(&mut collector),
                        );

                        // If it's pure fluid and not waterlogged, don't generate solid cube mesh
                        if meta.is_fluid && !meta.is_waterlogged {
                            continue;
                        }
                    }

                    // 2. Solid Block / Baked Model Meshing
                    let baked_opt = &palette_models[pal_idx];
                    let is_emissive = palette_emissive[pal_idx];

                    let get_padded_neighbor_state = |target_pos: IVec3| -> Option<&str> {
                        let rel_x = target_pos.x - block_pos.x + px as i32;
                        let rel_y = target_pos.y - block_pos.y + py as i32;
                        let rel_z = target_pos.z - block_pos.z + pz as i32;
                        if (0..18).contains(&rel_x) && (0..18).contains(&rel_y) && (0..18).contains(&rel_z) {
                            Some(padded.get_padded_state(rel_x as usize, rel_y as usize, rel_z as usize))
                        } else {
                            None
                        }
                    };

                    if let Some(_baked) = baked_opt {
                        let (culled_faces, unculled_faces) = match &palette_meshing_data[pal_idx] {
                            PaletteMeshingData::Model { culled_faces, unculled_faces } => (culled_faces, unculled_faces),
                            _ => unreachable!(),
                        };

                        let mut emit_model_face = |mf: &PreResolvedModelFace| {
                            let face = &mf.face;
                            let pre_face = &mf.pre;
                            let face_dir = face.direction;

                            // Calculate 4-corner AO for face.direction
                            let ao_levels = if config.enable_ao && !is_emissive {
                                calculate_face_ao(face_dir, |dx, dy, dz| {
                                    let sx = (px as i32 + dx) as usize;
                                    let sy = (py as i32 + dy) as usize;
                                    let sz = (pz as i32 + dz) as usize;
                                    if sx < 18 && sy < 18 && sz < 18 {
                                        is_opaque_fn(sx, sy, sz)
                                    } else {
                                        false
                                    }
                                })
                            } else {
                                [3, 3, 3, 3]
                            };

                            let (final_tex_key, override_uvs, mat_slot, chunk_id, tex_id, tint_data, tint_color, colormap_uv) =
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
                                        get_padded_neighbor_state,
                                    ) {
                                        if let Some(atlas) = &config.atlas_address_map {
                                            if let Some(atlas_loc) = atlas.lookup(&resolved_loc) {
                                                let u_min = atlas_loc.frame_0_uv_bounds[0];
                                                let v_min = atlas_loc.frame_0_uv_bounds[1];
                                                let u_span = atlas_loc.frame_0_uv_bounds[2] - u_min;
                                                let v_span = atlas_loc.frame_0_uv_bounds[3] - v_min;
                                                let remapped = [
                                                    glam::Vec2::new(
                                                        u_min + face.uvs[0].x * u_span,
                                                        v_min + (1.0 - face.uvs[0].y) * v_span,
                                                    ),
                                                    glam::Vec2::new(
                                                        u_min + face.uvs[1].x * u_span,
                                                        v_min + (1.0 - face.uvs[1].y) * v_span,
                                                    ),
                                                    glam::Vec2::new(
                                                        u_min + face.uvs[2].x * u_span,
                                                        v_min + (1.0 - face.uvs[2].y) * v_span,
                                                    ),
                                                    glam::Vec2::new(
                                                        u_min + face.uvs[3].x * u_span,
                                                        v_min + (1.0 - face.uvs[3].y) * v_span,
                                                    ),
                                                ];
                                                let ctm_key = resolved_loc.as_string();
                                                let clean_b = mtk_resource::extract_block_name(state_str);
                                                let (td, tc, c_uv) = compute_face_tint(
                                                    &ctm_key,
                                                    clean_b,
                                                    face.tint_index,
                                                    config.biome_resolver.as_deref(),
                                                );
                                                (
                                                    ctm_key,
                                                    Some(remapped),
                                                    atlas_loc.chunk_id,
                                                    atlas_loc.chunk_id as i32,
                                                    atlas_loc.texture_id,
                                                    td,
                                                    tc,
                                                    c_uv,
                                                )
                                            } else {
                                                (
                                                    pre_face.source_texture_key.clone(),
                                                    pre_face.override_uvs,
                                                    pre_face.mat_slot,
                                                    pre_face.chunk_id,
                                                    pre_face.tex_id,
                                                    pre_face.tint_data,
                                                    pre_face.tint_color,
                                                    pre_face.colormap_uv,
                                                )
                                            }
                                        } else {
                                            (
                                                pre_face.source_texture_key.clone(),
                                                pre_face.override_uvs,
                                                pre_face.mat_slot,
                                                pre_face.chunk_id,
                                                pre_face.tex_id,
                                                pre_face.tint_data,
                                                pre_face.tint_color,
                                                pre_face.colormap_uv,
                                            )
                                        }
                                    } else {
                                        (
                                            pre_face.source_texture_key.clone(),
                                            pre_face.override_uvs,
                                            pre_face.mat_slot,
                                            pre_face.chunk_id,
                                            pre_face.tex_id,
                                            pre_face.tint_data,
                                            pre_face.tint_color,
                                            pre_face.colormap_uv,
                                        )
                                    }
                                } else {
                                    (
                                        pre_face.source_texture_key.clone(),
                                        pre_face.override_uvs,
                                        pre_face.mat_slot,
                                        pre_face.chunk_id,
                                        pre_face.tex_id,
                                        pre_face.tint_data,
                                        pre_face.tint_color,
                                        pre_face.colormap_uv,
                                    )
                                };

                            emit_baked_face(
                                &mut mesh,
                                face,
                                wx,
                                wy,
                                wz,
                                ao_levels,
                                config,
                                override_uvs,
                                mat_slot,
                                &mut collector,
                                final_tex_key,
                                chunk_id,
                                tex_id,
                                tint_data,
                                tint_color,
                                colormap_uv,
                                block_pos,
                                face_dir,
                            );
                        };

                        // 1. Process 6 cull directions (Directional Fast-Path)
                        for dir in Direction::ALL {
                            let faces = &culled_faces[dir.to_index()];
                            if faces.is_empty() {
                                continue;
                            }

                            let offset = dir.offset();
                            let npx = (px as i32 + offset.x) as usize;
                            let npy = (py as i32 + offset.y) as usize;
                            let npz = (pz as i32 + offset.z) as usize;

                            let npal_idx = padded.padded_voxels[padded_index(npx, npy, npz)] as usize;
                            let n_meta = if npal_idx < palette_metas.len() {
                                Some(&*palette_metas[npal_idx])
                            } else {
                                None
                            };
                            let neighbor_pos = block_pos + offset;

                            if !culler.should_render_face(
                                meta,
                                n_meta,
                                dir,
                                None,
                                Some(block_pos),
                                Some(neighbor_pos),
                            ) {
                                continue;
                            }

                            for mf in faces {
                                emit_model_face(mf);
                            }
                        }

                        // 2. Process unculled / internal faces (zero neighbor lookup!)
                        for mf in unculled_faces {
                            emit_model_face(mf);
                        }
                    } else {
                        // Standard unit cube meshing
                        let cube_faces = match &palette_meshing_data[pal_idx] {
                            PaletteMeshingData::UnitCube { faces } => faces,
                            _ => unreachable!(),
                        };

                        for dir in Direction::ALL {
                            let offset = dir.offset();
                            let npx = (px as i32 + offset.x) as usize;
                            let npy = (py as i32 + offset.y) as usize;
                            let npz = (pz as i32 + offset.z) as usize;

                            let npal_idx = padded.padded_voxels[padded_index(npx, npy, npz)] as usize;
                            let n_meta = if npal_idx < palette_metas.len() {
                                Some(&*palette_metas[npal_idx])
                            } else {
                                None
                            };

                            let neighbor_pos = block_pos + offset;

                            // Check occlusion visibility
                            if !culler.should_render_face(
                                meta,
                                n_meta,
                                dir,
                                None,
                                Some(block_pos),
                                Some(neighbor_pos),
                            ) {
                                continue;
                            }

                            // Calculate 4-corner AO
                            let ao_levels = if config.enable_ao && !is_emissive {
                                calculate_face_ao(dir, |dx, dy, dz| {
                                    let sx = (px as i32 + dx) as usize;
                                    let sy = (py as i32 + dy) as usize;
                                    let sz = (pz as i32 + dz) as usize;
                                    if sx < 18 && sy < 18 && sz < 18 {
                                        is_opaque_fn(sx, sy, sz)
                                    } else {
                                        false
                                    }
                                })
                            } else {
                                [3, 3, 3, 3]
                            };

                            let pre_face = &cube_faces[dir.to_index()];

                            let (final_tex_key, override_uvs, mat_slot, chunk_id, tex_id, tint_data, tint_color, colormap_uv) =
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
                                        get_padded_neighbor_state,
                                    ) {
                                        if let Some(atlas) = &config.atlas_address_map {
                                            if let Some(atlas_loc) = atlas.lookup(&resolved_loc) {
                                                let u_min = atlas_loc.frame_0_uv_bounds[0];
                                                let v_min = atlas_loc.frame_0_uv_bounds[1];
                                                let u_max = atlas_loc.frame_0_uv_bounds[2];
                                                let v_max = atlas_loc.frame_0_uv_bounds[3];
                                                let remapped = [
                                                    glam::Vec2::new(u_min, v_max),
                                                    glam::Vec2::new(u_min, v_min),
                                                    glam::Vec2::new(u_max, v_min),
                                                    glam::Vec2::new(u_max, v_max),
                                                ];
                                                let ctm_key = resolved_loc.as_string();
                                                let clean_b = mtk_resource::extract_block_name(state_str);
                                                let (td, tc, c_uv) = compute_face_tint(
                                                    &ctm_key,
                                                    clean_b,
                                                    -1,
                                                    config.biome_resolver.as_deref(),
                                                );
                                                (
                                                    ctm_key,
                                                    Some(remapped),
                                                    atlas_loc.chunk_id,
                                                    atlas_loc.chunk_id as i32,
                                                    atlas_loc.texture_id,
                                                    td,
                                                    tc,
                                                    c_uv,
                                                )
                                            } else {
                                                (
                                                    pre_face.source_texture_key.clone(),
                                                    pre_face.override_uvs,
                                                    pre_face.mat_slot,
                                                    pre_face.chunk_id,
                                                    pre_face.tex_id,
                                                    pre_face.tint_data,
                                                    pre_face.tint_color,
                                                    pre_face.colormap_uv,
                                                )
                                            }
                                        } else {
                                            (
                                                pre_face.source_texture_key.clone(),
                                                pre_face.override_uvs,
                                                pre_face.mat_slot,
                                                pre_face.chunk_id,
                                                pre_face.tex_id,
                                                pre_face.tint_data,
                                                pre_face.tint_color,
                                                pre_face.colormap_uv,
                                            )
                                        }
                                    } else {
                                        (
                                            pre_face.source_texture_key.clone(),
                                            pre_face.override_uvs,
                                            pre_face.mat_slot,
                                            pre_face.chunk_id,
                                            pre_face.tex_id,
                                            pre_face.tint_data,
                                            pre_face.tint_color,
                                            pre_face.colormap_uv,
                                        )
                                    }
                                } else {
                                    (
                                        pre_face.source_texture_key.clone(),
                                        pre_face.override_uvs,
                                        pre_face.mat_slot,
                                        pre_face.chunk_id,
                                        pre_face.tex_id,
                                        pre_face.tint_data,
                                        pre_face.tint_color,
                                        pre_face.colormap_uv,
                                    )
                                };

                            emit_unit_cube_face(
                                &mut mesh,
                                dir,
                                wx,
                                wy,
                                wz,
                                ao_levels,
                                config,
                                override_uvs,
                                mat_slot,
                                &mut collector,
                                final_tex_key,
                                chunk_id,
                                tex_id,
                                tint_data,
                                tint_color,
                                colormap_uv,
                                block_pos,
                            );
                        }
                    }
                }
            }
        }

        collector.attach_to_mesh(&mut mesh);
        if config.weld_vertices {
            mesh.weld_spatial_vertices(1e-4);
        }
        mesh
    }

    /// Meshes a batch of `PaddedVoxelArray`s.
    ///
    /// - When `feature = "parallel"` is enabled: parallelizes across worker threads using Rayon.
    /// - When compiled for WASM or single-threaded mode: falls back to sequential iteration.
    pub fn mesh_sections<F>(
        sections: &[PaddedVoxelArray],
        culler: &FaceCuller,
        model_provider: F,
        config: &MesherConfig,
    ) -> Result<Vec<(IVec3, MeshData)>, VoxelError>
    where
        F: Fn(&str) -> Option<Arc<BakedModel>> + Sync + Send,
    {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            let results = sections
                .par_iter()
                .map(|sec| {
                    let mesh = Self::mesh_section(sec, culler, |st| model_provider(st), config);
                    (sec.coord, mesh)
                })
                .collect();
            Ok(results)
        }

        #[cfg(not(feature = "parallel"))]
        {
            let results = sections
                .iter()
                .map(|sec| {
                    let mesh = Self::mesh_section(sec, culler, |st| model_provider(st), config);
                    (sec.coord, mesh)
                })
                .collect();
            Ok(results)
        }
    }

    /// Meshes a batch of `PaddedVoxelArray`s with optional explicit thread pool count.
    pub fn mesh_sections_parallel<F>(
        sections: &[PaddedVoxelArray],
        culler: &FaceCuller,
        model_provider: F,
        config: &MesherConfig,
        num_threads: Option<usize>,
    ) -> Result<Vec<(IVec3, MeshData)>, VoxelError>
    where
        F: Fn(&str) -> Option<Arc<BakedModel>> + Sync + Send,
    {
        mtk_core::constants::concurrency::execute_parallel(num_threads, || {
            Self::mesh_sections(sections, culler, model_provider, config)
        })
        .map_err(VoxelError::ThreadPoolError)?
    }
}

/// Helper to emit a baked face into `MeshData` with AO shading and anisotropy flip.
#[inline]
fn emit_baked_face(
    mesh: &mut MeshData,
    face: &BakedFace,
    wx: f32,
    wy: f32,
    wz: f32,
    ao_levels: [u8; 4],
    config: &MesherConfig,
    override_uvs: Option<[glam::Vec2; 4]>,
    mat_slot: u16,
    collector: &mut FaceAttributesCollector,
    source_texture_key: String,
    atlas_chunk_id: i32,
    atlas_texture_id: u32,
    tint_data: [f32; 4],
    tint_color: [f32; 4],
    colormap_uv: [f32; 3],
    block_pos: IVec3,
    dir: Direction,
) {
    let base_idx = mesh.positions.len() as u32;
    let norm = config.transform_direction(face.normal);
    let n = [norm.x, norm.y, norm.z];

    let colors = mesh.colors.get_or_insert_with(Vec::new);

    for i in 0..4 {
        let v = face.vertices[i];
        let p = config.transform_position(Vec3::new(wx + v.x, wy + v.y, wz + v.z));
        mesh.positions.push([p.x, p.y, p.z]);
        mesh.normals.push(n);
        if let Some(ref uvs) = override_uvs {
            mesh.uvs.push([uvs[i].x, uvs[i].y]);
        } else {
            mesh.uvs.push([face.uvs[i].x, 1.0 - face.uvs[i].y]);
        }

        let ao_b = ao_level_to_brightness(ao_levels[i]);
        colors.push([ao_b, ao_b, ao_b, 1.0]);
    }

    mesh.quad_indices
        .get_or_insert_with(Vec::new)
        .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx + 3]);

    // Triangulate with anisotropy diagonal flip
    if should_flip_quad_diagonal(ao_levels) {
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);
    } else {
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 3);
    }

    mesh.face_materials.push(mat_slot);
    mesh.face_tint_indices.push(face.tint_index);

    collector.push_face(
        source_texture_key,
        mat_slot as i32,
        atlas_chunk_id,
        atlas_texture_id,
        [1.0, 1.0, 0.0, 0.0],
        face.uv_rot,
        0,
        tint_data,
        tint_color,
        colormap_uv,
        block_pos,
        dir.to_index() as u8,
    );
}

/// Helper to emit a unit cube face into `MeshData` with AO shading and anisotropy flip.
#[inline]
fn emit_unit_cube_face(
    mesh: &mut MeshData,
    dir: Direction,
    wx: f32,
    wy: f32,
    wz: f32,
    ao_levels: [u8; 4],
    config: &MesherConfig,
    override_uvs: Option<[glam::Vec2; 4]>,
    mat_slot: u16,
    collector: &mut FaceAttributesCollector,
    source_texture_key: String,
    atlas_chunk_id: i32,
    atlas_texture_id: u32,
    tint_data: [f32; 4],
    tint_color: [f32; 4],
    colormap_uv: [f32; 3],
    block_pos: IVec3,
) {
    let base_idx = mesh.positions.len() as u32;
    let norm = config.transform_direction(dir.normal());
    let n = [norm.x, norm.y, norm.z];

    let (v0, v1, v2, v3) = match dir {
        Direction::East => (
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ),
        Direction::West => (
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ),
        Direction::Up => (
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
        ),
        Direction::Down => (
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ),
        Direction::South => (
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ),
        Direction::North => (
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ),
    };

    let colors = mesh.colors.get_or_insert_with(Vec::new);

    for (i, v) in [v0, v1, v2, v3].into_iter().enumerate() {
        let p = config.transform_position(Vec3::new(wx + v.x, wy + v.y, wz + v.z));
        mesh.positions.push([p.x, p.y, p.z]);
        mesh.normals.push(n);

        let ao_b = ao_level_to_brightness(ao_levels[i]);
        colors.push([ao_b, ao_b, ao_b, 1.0]);
    }

    mesh.quad_indices
        .get_or_insert_with(Vec::new)
        .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx + 3]);

    if let Some(ref uvs) = override_uvs {
        for uv in uvs {
            mesh.uvs.push([uv.x, uv.y]);
        }
    } else {
        mesh.uvs.push([0.0, 1.0]);
        mesh.uvs.push([0.0, 0.0]);
        mesh.uvs.push([1.0, 0.0]);
        mesh.uvs.push([1.0, 1.0]);
    }

    if should_flip_quad_diagonal(ao_levels) {
        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);
    } else {
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 2);
        mesh.indices.push(base_idx + 3);

        mesh.indices.push(base_idx);
        mesh.indices.push(base_idx + 1);
        mesh.indices.push(base_idx + 3);
    }

    mesh.face_materials.push(mat_slot);
    mesh.face_tint_indices.push(-1);

    collector.push_face(
        source_texture_key,
        mat_slot as i32,
        atlas_chunk_id,
        atlas_texture_id,
        [1.0, 1.0, 0.0, 0.0],
        0.0,
        0,
        tint_data,
        tint_color,
        colormap_uv,
        block_pos,
        dir.to_index() as u8,
    );
}
