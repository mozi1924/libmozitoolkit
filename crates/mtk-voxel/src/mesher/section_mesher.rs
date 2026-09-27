use std::sync::Arc;

use glam::{IVec3, Vec3};
use mtk_core::attributes::{AttributeData, AttributeDomain, MeshAttribute};
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

/// High-throughput collector for all 15 per-face attributes needed for Atlas materials,
/// UV tiling transforms, labPBR provenance, and dynamic Biome Tinting.
#[derive(Debug, Clone, Default)]
pub struct FaceAttributesCollector {
    pub source_texture_keys: Vec<String>,
    pub material_slots: Vec<i32>,
    pub atlas_chunk_ids: Vec<i32>,
    pub atlas_texture_ids: Vec<u32>,
    pub uv_tiling_transforms: Vec<[f32; 4]>,
    pub uv_transforms: Vec<[f32; 4]>,
    pub uv_rotations: Vec<f32>,
    pub uv_modes: Vec<u8>,
    pub biome_tint_data: Vec<[f32; 4]>,
    pub biome_tint_colors: Vec<[f32; 4]>,
    pub colormap_uvs: Vec<[f32; 3]>,
    pub block_xs: Vec<i32>,
    pub block_ys: Vec<i32>,
    pub block_zs: Vec<i32>,
    pub face_dirs: Vec<u8>,
}

impl FaceAttributesCollector {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            source_texture_keys: Vec::with_capacity(capacity),
            material_slots: Vec::with_capacity(capacity),
            atlas_chunk_ids: Vec::with_capacity(capacity),
            atlas_texture_ids: Vec::with_capacity(capacity),
            uv_tiling_transforms: Vec::with_capacity(capacity),
            uv_transforms: Vec::with_capacity(capacity),
            uv_rotations: Vec::with_capacity(capacity),
            uv_modes: Vec::with_capacity(capacity),
            biome_tint_data: Vec::with_capacity(capacity),
            biome_tint_colors: Vec::with_capacity(capacity),
            colormap_uvs: Vec::with_capacity(capacity),
            block_xs: Vec::with_capacity(capacity),
            block_ys: Vec::with_capacity(capacity),
            block_zs: Vec::with_capacity(capacity),
            face_dirs: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn push_face(
        &mut self,
        source_texture_key: String,
        material_slot: i32,
        atlas_chunk_id: i32,
        atlas_texture_id: u32,
        uv_tiling_transform: [f32; 4],
        uv_rotation: f32,
        uv_mode: u8,
        tint_data: [f32; 4],
        tint_color: [f32; 4],
        colormap_uv: [f32; 3],
        block_pos: IVec3,
        face_dir: u8,
    ) {
        self.source_texture_keys.push(source_texture_key);
        self.material_slots.push(material_slot);
        self.atlas_chunk_ids.push(atlas_chunk_id);
        self.atlas_texture_ids.push(atlas_texture_id);
        self.uv_tiling_transforms.push(uv_tiling_transform);
        self.uv_transforms.push(uv_tiling_transform);
        self.uv_rotations.push(uv_rotation);
        self.uv_modes.push(uv_mode);
        self.biome_tint_data.push(tint_data);
        self.biome_tint_colors.push(tint_color);
        self.colormap_uvs.push(colormap_uv);
        self.block_xs.push(block_pos.x);
        self.block_ys.push(block_pos.y);
        self.block_zs.push(block_pos.z);
        self.face_dirs.push(face_dir);
    }

    pub fn attach_to_mesh(self, mesh: &mut MeshData) {
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_source_texture_key",
            AttributeDomain::Face,
            AttributeData::String(self.source_texture_keys),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_material_slot",
            AttributeDomain::Face,
            AttributeData::Int32(self.material_slots),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_atlas_chunk_id",
            AttributeDomain::Face,
            AttributeData::Int32(self.atlas_chunk_ids),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_atlas_texture_id",
            AttributeDomain::Face,
            AttributeData::UInt32(self.atlas_texture_ids),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_tiling_transform",
            AttributeDomain::Face,
            AttributeData::Float4(self.uv_tiling_transforms),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_transform",
            AttributeDomain::Face,
            AttributeData::Float4(self.uv_transforms),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_rotation",
            AttributeDomain::Face,
            AttributeData::Float(self.uv_rotations),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_mode",
            AttributeDomain::Face,
            AttributeData::UInt8(self.uv_modes),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_tint_data",
            AttributeDomain::Face,
            AttributeData::Float4(self.biome_tint_data),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_tint_color",
            AttributeDomain::Face,
            AttributeData::Float4(self.biome_tint_colors),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_colormap_uv",
            AttributeDomain::Face,
            AttributeData::Float3(self.colormap_uvs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_x",
            AttributeDomain::Face,
            AttributeData::Int32(self.block_xs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_y",
            AttributeDomain::Face,
            AttributeData::Int32(self.block_ys),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_z",
            AttributeDomain::Face,
            AttributeData::Int32(self.block_zs),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_face_dir",
            AttributeDomain::Face,
            AttributeData::UInt8(self.face_dirs),
        ));
    }
}

/// Generates directional candidate texture identifiers for unit cube blocks.
pub fn get_unit_cube_texture_candidates(clean_block: &str, dir: Direction) -> Vec<String> {
    let mut candidates = Vec::with_capacity(4);
    let is_top = dir == Direction::Up;
    let is_bottom = dir == Direction::Down;
    let is_side = !is_top && !is_bottom;

    if clean_block == "grass_block" || clean_block == "grass" {
        if is_top {
            candidates.push("block/grass_block_top".to_string());
            candidates.push("block/grass_top".to_string());
        } else if is_bottom {
            candidates.push("block/dirt".to_string());
        } else {
            candidates.push("block/grass_block_side".to_string());
            candidates.push("block/grass_side".to_string());
        }
    } else if clean_block == "podzol" {
        if is_top {
            candidates.push("block/podzol_top".to_string());
        } else if is_bottom {
            candidates.push("block/dirt".to_string());
        } else {
            candidates.push("block/podzol_side".to_string());
        }
    } else if clean_block == "mycelium" {
        if is_top {
            candidates.push("block/mycelium_top".to_string());
        } else if is_bottom {
            candidates.push("block/dirt".to_string());
        } else {
            candidates.push("block/mycelium_side".to_string());
        }
    } else if clean_block.contains("log")
        || clean_block.contains("wood")
        || clean_block.contains("pillar")
        || clean_block.contains("stem")
        || clean_block.contains("hyphae")
    {
        if is_top || is_bottom {
            candidates.push(format!("block/{}_top", clean_block));
            candidates.push(format!("block/{}_end", clean_block));
        } else {
            candidates.push(format!("block/{}_side", clean_block));
            candidates.push(format!("block/{}", clean_block));
        }
    } else {
        if is_top {
            candidates.push(format!("block/{}_top", clean_block));
            candidates.push(format!("block/{}_up", clean_block));
            candidates.push(format!("block/{}_end", clean_block));
        } else if is_bottom {
            candidates.push(format!("block/{}_bottom", clean_block));
            candidates.push(format!("block/{}_down", clean_block));
            candidates.push(format!("block/{}_end", clean_block));
        } else if is_side {
            candidates.push(format!("block/{}_side", clean_block));
            candidates.push(format!("block/{}_front", clean_block));
        }
        candidates.push(format!("block/{}", clean_block));
    }

    candidates
}

/// Computes biome tint data and linear color for a face.
fn compute_face_tint(
    texture_key: &str,
    block_name: &str,
    tint_index: i16,
    biome_resolver: Option<&mtk_material::BiomeResolver>,
) -> ([f32; 4], [f32; 4], [f32; 3]) {
    let default_pal = mtk_material::biome::get_biome_palette("plains");
    let default_uv = default_pal.colormap_uv();
    let colormap_uv_3 = [default_uv[0], default_uv[1], 0.0];

    if let Some(resolver) = biome_resolver {
        let t_idx = if tint_index >= 0 { Some(tint_index as i32) } else { None };
        let info = resolver.get_tint_info(texture_key, Some(block_name), t_idx);
        let tw = info.tint_weight;
        let bw = info.base_tint_weight;
        let ow = info.overlay_tint_weight;
        let tt = info.tint_type;
        let is_hc = info.is_hardcoded;

        let packed_data = [bw, ow, tw, tt as f32];
        let final_col = match tt {
            mtk_material::TINT_TYPE_GRASS => default_pal.grass_linear(),
            mtk_material::TINT_TYPE_FOLIAGE => default_pal.foliage_linear(),
            mtk_material::TINT_TYPE_DRY_FOLIAGE => default_pal.dry_foliage_linear(),
            mtk_material::TINT_TYPE_WATER => default_pal.water_linear(),
            mtk_material::TINT_TYPE_HARDCODED => {
                info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0])
            }
            _ => {
                if is_hc {
                    info.hardcoded_color.unwrap_or([1.0, 1.0, 1.0, 1.0])
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                }
            }
        };
        (packed_data, final_col, colormap_uv_3)
    } else if tint_index >= 0 {
        let packed_data = [1.0, 1.0, 1.0, 1.0]; // default grass tint
        (packed_data, default_pal.grass_linear(), colormap_uv_3)
    } else {
        let packed_data = [1.0, 1.0, 0.0, 0.0]; // no tint
        (packed_data, [1.0, 1.0, 1.0, 1.0], colormap_uv_3)
    }
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
                            Some(&mut collector),
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
                                    is_opaque_fn(sx, sy, sz)
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

                                    let (final_tex_key, override_uvs, mat_slot, chunk_id, tex_id) =
                                        if let Some(atlas) = &config.atlas_address_map {
                                            let resolved = if let Some(ref loc) = final_loc {
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

                                    let clean_block = mtk_resource::extract_block_name(state_str);
                                    let (tint_data, tint_color, colormap_uv) = compute_face_tint(
                                        &final_tex_key,
                                        clean_block,
                                        face.tint_index,
                                        config.biome_resolver.as_deref(),
                                    );

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
                                        dir,
                                    );
                                }
                            }
                        } else {
                            let clean_block = mtk_resource::extract_block_name(state_str)
                                .strip_prefix("minecraft:")
                                .unwrap_or(mtk_resource::extract_block_name(state_str));

                            let (final_tex_key, override_uvs, mat_slot, chunk_id, tex_id) =
                                if let Some(atlas) = &config.atlas_address_map {
                                    let mut resolved = None;

                                    if let Some(solver) = &config.ctm_solver {
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
                                            resolved = atlas
                                                .lookup(&resolved_loc)
                                                .map(|sp| (resolved_loc, sp));
                                        }
                                    }

                                    if resolved.is_none() {
                                        let candidates =
                                            get_unit_cube_texture_candidates(clean_block, dir);
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
                                    }

                                    if let Some((res_loc, atlas_loc)) = resolved {
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
                                        (
                                            res_loc.as_string(),
                                            Some(remapped),
                                            atlas_loc.chunk_id,
                                            atlas_loc.chunk_id as i32,
                                            atlas_loc.texture_id,
                                        )
                                    } else {
                                        (
                                            format!("minecraft:block/{}", clean_block),
                                            None,
                                            0,
                                            0,
                                            0,
                                        )
                                    }
                                } else {
                                    (
                                        format!("minecraft:block/{}", clean_block),
                                        None,
                                        0,
                                        0,
                                        0,
                                    )
                                };

                            let (tint_data, tint_color, colormap_uv) = compute_face_tint(
                                &final_tex_key,
                                clean_block,
                                if (clean_block == "grass_block" || clean_block == "grass")
                                    && dir == Direction::Up
                                {
                                    0
                                } else {
                                    -1
                                },
                                config.biome_resolver.as_deref(),
                            );

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
    let norm = config.transform_coord(face.normal);
    let n = [norm.x, norm.y, norm.z];

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
    let norm = config.transform_coord(dir.normal());
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
