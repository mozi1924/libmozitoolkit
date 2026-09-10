use std::sync::Arc;

use glam::{IVec3, Vec3};
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use mtk_cull::types::BlockCullMeta;
use mtk_cull::FaceCuller;
use mtk_model::baked::{BakedFace, BakedModel};
use mtk_model::baker::is_block_emissive;
use mtk_model::blockstate::BlockState;

use crate::ao::{ao_level_to_brightness, calculate_face_ao, should_flip_quad_diagonal};
use crate::fluid::emit_fluid_geometry;
use crate::storage::{padded_index, PaddedVoxelArray, SECTION_SIZE};
use crate::types::{MesherConfig, VoxelError};

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

        // Pre-resolve palette metadata
        let palette_metas: Vec<Arc<BlockCullMeta>> = padded
            .palette
            .iter()
            .map(|st| culler.get_meta(st, None, None))
            .collect();

        // Pre-resolve baked models
        let mut palette_models: Vec<Option<Arc<BakedModel>>> =
            Vec::with_capacity(padded.palette.len());
        for st in &padded.palette {
            palette_models.push(get_baked_model(st));
        }

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
                        );

                        // If it's pure fluid and not waterlogged, don't generate solid cube mesh
                        if meta.is_fluid && !meta.is_waterlogged {
                            continue;
                        }
                    }

                    // 2. Solid Block / Baked Model Meshing
                    let baked_opt = &palette_models[pal_idx];
                    let blockstate = BlockState::parse(state_str);
                    let is_emissive = blockstate.as_ref().map(is_block_emissive).unwrap_or(false);

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
                                    is_opaque_fn(sx, sz, sy)
                                } else {
                                    false
                                }
                            })
                        } else {
                            [3, 3, 3, 3]
                        };

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

                        if let Some(baked) = baked_opt {
                            for el in &baked.elements {
                                if let Some(face) = el.faces.get(&dir) {
                                    let base_loc = if !face.texture.is_empty() {
                                        mtk_resource::ResourceLocation::parse(&face.texture).ok()
                                    } else {
                                        None
                                    };

                                    let resolved_loc = if let Some(solver) = &config.ctm_solver {
                                        solver.resolve_face(
                                            state_str,
                                            dir,
                                            block_pos,
                                            base_loc.as_ref(),
                                            None,
                                            get_padded_neighbor_state,
                                        )
                                    } else {
                                        None
                                    };

                                    let final_loc = resolved_loc.as_ref().or(base_loc.as_ref());

                                    let (override_uvs, mat_slot) = if let (Some(atlas), Some(loc)) =
                                        (&config.atlas_address_map, final_loc)
                                    {
                                        if let Some(atlas_loc) = atlas.lookup(loc) {
                                            let u_min = atlas_loc.frame_0_uv_bounds[0];
                                            let v_min = atlas_loc.frame_0_uv_bounds[1];
                                            let u_span = atlas_loc.frame_0_uv_bounds[2] - u_min;
                                            let v_span = atlas_loc.frame_0_uv_bounds[3] - v_min;
                                            let remapped = [
                                                glam::Vec2::new(
                                                    u_min + face.uvs[0].x * u_span,
                                                    v_min + face.uvs[0].y * v_span,
                                                ),
                                                glam::Vec2::new(
                                                    u_min + face.uvs[1].x * u_span,
                                                    v_min + face.uvs[1].y * v_span,
                                                ),
                                                glam::Vec2::new(
                                                    u_min + face.uvs[2].x * u_span,
                                                    v_min + face.uvs[2].y * v_span,
                                                ),
                                                glam::Vec2::new(
                                                    u_min + face.uvs[3].x * u_span,
                                                    v_min + face.uvs[3].y * v_span,
                                                ),
                                            ];
                                            (Some(remapped), atlas_loc.chunk_id)
                                        } else {
                                            (None, 0)
                                        }
                                    } else {
                                        (None, 0)
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
                                    );
                                }
                            }
                        } else {
                            let clean_block = mtk_resource::extract_block_name(state_str)
                                .strip_prefix("minecraft:")
                                .unwrap_or(mtk_resource::extract_block_name(state_str));
                            let base_loc = mtk_resource::ResourceLocation::new(
                                "minecraft",
                                format!("block/{}", clean_block),
                            );

                            let resolved_loc = if let Some(solver) = &config.ctm_solver {
                                solver.resolve_face(
                                    state_str,
                                    dir,
                                    block_pos,
                                    Some(&base_loc),
                                    None,
                                    get_padded_neighbor_state,
                                )
                            } else {
                                None
                            };

                            let final_loc = resolved_loc.as_ref().unwrap_or(&base_loc);

                            let (override_uvs, mat_slot) = if let Some(atlas) = &config.atlas_address_map {
                                if let Some(atlas_loc) = atlas.lookup(final_loc) {
                                    let u_min = atlas_loc.frame_0_uv_bounds[0];
                                    let v_min = atlas_loc.frame_0_uv_bounds[1];
                                    let u_max = atlas_loc.frame_0_uv_bounds[2];
                                    let v_max = atlas_loc.frame_0_uv_bounds[3];
                                    let remapped = [
                                        glam::Vec2::new(u_min, v_min),
                                        glam::Vec2::new(u_min, v_max),
                                        glam::Vec2::new(u_max, v_max),
                                        glam::Vec2::new(u_max, v_min),
                                    ];
                                    (Some(remapped), atlas_loc.chunk_id)
                                } else {
                                    (None, 0)
                                }
                            } else {
                                (None, 0)
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
                            );
                        }
                    }
                }
            }
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
) {
    let base_idx = mesh.positions.len() as u32;
    let n = [face.normal.x, face.normal.y, face.normal.z];

    let colors = mesh.colors.get_or_insert_with(Vec::new);

    for i in 0..4 {
        let v = face.vertices[i];
        let p = config.transform_coord(Vec3::new(wx + v.x, wy + v.y, wz + v.z));
        mesh.positions.push([p.x, p.y, p.z]);
        mesh.normals.push(n);
        if let Some(ref uvs) = override_uvs {
            mesh.uvs.push([uvs[i].x, uvs[i].y]);
        } else {
            mesh.uvs.push([face.uvs[i].x, face.uvs[i].y]);
        }

        let ao_b = ao_level_to_brightness(ao_levels[i]);
        colors.push([ao_b, ao_b, ao_b, 1.0]);
    }

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
) {
    let base_idx = mesh.positions.len() as u32;
    let norm = dir.normal();
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
        let p = config.transform_coord(Vec3::new(wx + v.x, wy + v.y, wz + v.z));
        mesh.positions.push([p.x, p.y, p.z]);
        mesh.normals.push(n);

        let ao_b = ao_level_to_brightness(ao_levels[i]);
        colors.push([ao_b, ao_b, ao_b, 1.0]);
    }

    if let Some(ref uvs) = override_uvs {
        for uv in uvs {
            mesh.uvs.push([uv.x, uv.y]);
        }
    } else {
        mesh.uvs.push([0.0, 0.0]);
        mesh.uvs.push([0.0, 1.0]);
        mesh.uvs.push([1.0, 1.0]);
        mesh.uvs.push([1.0, 0.0]);
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
}
