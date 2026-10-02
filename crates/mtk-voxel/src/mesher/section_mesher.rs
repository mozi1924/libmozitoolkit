//! High-performance mesh generator for individual and batch chunk sections.

use std::sync::Arc;
use glam::IVec3;
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use mtk_cull::types::BlockCullMeta;
use mtk_cull::FaceCuller;
use mtk_model::baked::BakedModel;
use mtk_model::baker::is_block_emissive;
use mtk_model::blockstate::BlockState;

use crate::ao::calculate_face_ao;
use crate::fluid::emit_fluid_geometry;
use crate::storage::{padded_index, PaddedVoxelArray, SECTION_SIZE};
use crate::types::{MesherConfig, VoxelError};

use super::collector::FaceAttributesCollector;
use super::emitter::{emit_baked_face, emit_unit_cube_face};
use super::shading::{
    build_palette_meshing_data, resolve_model_face_shading, resolve_unit_cube_face_shading,
    sample_biome_tint, PaletteMeshingData, PreResolvedModelFace,
};

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
                let runtime_meta = culler.get_meta(st, None, None);
                if let Some(model) = model_opt {
                    let mut meta = model.get_or_compute_cull_meta();
                    // Runtime states in voxel storage (e.g. waterlogged) must take precedence
                    // over baked model base state cull_meta
                    if runtime_meta.is_waterlogged {
                        meta.is_waterlogged = true;
                    }
                    meta.props = runtime_meta.props.clone();
                    Arc::new(meta)
                } else {
                    runtime_meta
                }
            })
            .collect();

        // Pre-resolve palette emissive states
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

        // Pre-resolve palette meshing data outside hot loop
        let palette_meshing_data = build_palette_meshing_data(padded, &palette_models, config);

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

                        let col_idx = lx * 16 + lz;
                        let (fluid_col, fluid_uv) = if let Some(ref biome_cols) = padded.biome_data {
                            if col_idx < biome_cols.len() {
                                let col = &biome_cols[col_idx];
                                (Some(col.water_color), Some([col.colormap_uv[0], col.colormap_uv[1], 0.0]))
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };

                        let effective_fluid_state = if meta.is_waterlogged && !meta.is_fluid {
                            "minecraft:water[level=0]"
                        } else {
                            state_str
                        };

                        emit_fluid_geometry(
                            &mut mesh,
                            block_pos.x,
                            block_pos.y,
                            block_pos.z,
                            wx,
                            wy,
                            wz,
                            effective_fluid_state,
                            get_state,
                            culler,
                            config,
                            0,
                            Some(&mut collector),
                            fluid_col,
                            fluid_uv,
                        );

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

                            let shading = resolve_model_face_shading(
                                mf,
                                state_str,
                                block_pos,
                                config,
                                get_padded_neighbor_state,
                            );

                            let (final_tint_color, final_colormap_uv) = sample_biome_tint(
                                padded.biome_data.as_deref(),
                                lx * 16 + lz,
                                shading.tint_data,
                                shading.tint_color,
                                shading.colormap_uv,
                            );

                            emit_baked_face(
                                &mut mesh,
                                face,
                                wx,
                                wy,
                                wz,
                                ao_levels,
                                config,
                                &mut collector,
                                &shading,
                                final_tint_color,
                                final_colormap_uv,
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
                            let shading = resolve_unit_cube_face_shading(
                                pre_face,
                                dir,
                                state_str,
                                block_pos,
                                config,
                                get_padded_neighbor_state,
                            );

                            let (final_tint_color, final_colormap_uv) = sample_biome_tint(
                                padded.biome_data.as_deref(),
                                lx * 16 + lz,
                                shading.tint_data,
                                shading.tint_color,
                                shading.colormap_uv,
                            );

                            emit_unit_cube_face(
                                &mut mesh,
                                dir,
                                wx,
                                wy,
                                wz,
                                ao_levels,
                                config,
                                &mut collector,
                                &shading,
                                final_tint_color,
                                final_colormap_uv,
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
