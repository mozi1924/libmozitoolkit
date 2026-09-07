use std::collections::HashMap;
use std::sync::Arc;

use glam::{IVec3, Vec3};
use mtk_core::direction::Direction;
use mtk_core::mesh::MeshData;
use mtk_cull::{BlockCullMeta, FaceCuller};

use crate::baked::{BakedFace, BakedModel};
use crate::error::ModelError;



/// Dimension of standard Minecraft sub-chunk section.
pub const SECTION_SIZE: usize = 16;
/// Total number of voxels in a 16x16x16 section.
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE; // 4096
/// Padded dimension with 1-voxel apron on each side for branchless boundary lookups.
pub const PADDED_SIZE: usize = 18;
/// Total number of voxels in a 18x18x18 padded volume.
pub const PADDED_VOLUME: usize = PADDED_SIZE * PADDED_SIZE * PADDED_SIZE; // 5832

/// Canonical local Minecraft voxel corner vertices in Blender space (origin at block center, or [0..1]).
pub fn mc_local_to_blender(lx: f32, ly: f32, lz: f32) -> Vec3 {
    Vec3::new(lx - 0.5, -(lz - 0.5), ly - 0.5)
}

/// Standard 16x16x16 voxel section with palette-indexed compression and optional 18x18x18 padded apron.
#[derive(Debug, Clone)]
pub struct VoxelSection {
    /// Section coordinate in chunk grid (e.g. `[cx, cy, cz]`).
    pub coord: IVec3,
    /// Palette mapping index `u16` -> BlockState string (e.g. 0 -> "minecraft:air").
    pub palette: Vec<String>,
    /// 18x18x18 padded block indices (`[x + 1][y + 1][z + 1]` in `0..18`).
    /// `pad_index = (x + 1) * 324 + (y + 1) * 18 + (z + 1)`.
    pub padded_voxels: Vec<u16>,
    /// Whether this section is known to be entirely air (empty).
    pub is_empty: bool,
}

impl VoxelSection {
    /// Creates an empty section at coordinate `coord`.
    pub fn new_empty(coord: IVec3) -> Self {
        Self {
            coord,
            palette: vec!["minecraft:air".to_string()],
            padded_voxels: vec![0; PADDED_VOLUME],
            is_empty: true,
        }
    }

    /// Creates a section from a flat 16x16x16 block array with a boundary neighbor sampler.
    ///
    /// - `coord`: Chunk section coordinate `(cx, cy, cz)`.
    /// - `core_blocks`: 4096-element array of blockstate strings at `(x, y, z)` in `0..16`.
    /// - `get_neighbor`: Closure querying blockstate string at local offset `(-1..17, -1..17, -1..17)`.
    pub fn from_flat_array<F>(
        coord: IVec3,
        core_blocks: &[String],
        mut get_neighbor: F,
    ) -> Self
    where
        F: FnMut(i32, i32, i32) -> String,
    {
        let mut palette_map: HashMap<String, u16> = HashMap::new();
        let mut palette: Vec<String> = Vec::new();

        let mut get_or_insert_palette = |name: &str| -> u16 {
            if let Some(&idx) = palette_map.get(name) {
                idx
            } else {
                let idx = palette.len() as u16;
                palette.push(name.to_string());
                palette_map.insert(name.to_string(), idx);
                idx
            }
        };

        // Pre-insert air as index 0
        get_or_insert_palette("minecraft:air");

        let mut padded_voxels = vec![0u16; PADDED_VOLUME];
        let mut non_air_count = 0;

        for px in 0..PADDED_SIZE {
            let lx = px as i32 - 1;
            for py in 0..PADDED_SIZE {
                let ly = py as i32 - 1;
                for pz in 0..PADDED_SIZE {
                    let lz = pz as i32 - 1;
                    let pad_idx = px * 324 + py * 18 + pz;

                    let state = if lx >= 0 && lx < 16 && ly >= 0 && ly < 16 && lz >= 0 && lz < 16 {
                        let core_idx = lx as usize * 256 + ly as usize * 16 + lz as usize;
                        if core_idx < core_blocks.len() {
                            &core_blocks[core_idx]
                        } else {
                            "minecraft:air"
                        }
                    } else {
                        &get_neighbor(lx, ly, lz)
                    };

                    let pal_idx = get_or_insert_palette(state);
                    padded_voxels[pad_idx] = pal_idx;

                    if lx >= 0 && lx < 16 && ly >= 0 && ly < 16 && lz >= 0 && lz < 16 {
                        if state != "minecraft:air" && state != "air" && !state.is_empty() {
                            non_air_count += 1;
                        }
                    }
                }
            }
        }

        Self {
            coord,
            palette,
            padded_voxels,
            is_empty: non_air_count == 0,
        }
    }

    /// Retrieves block state string at padded coordinate `px, py, pz` (0..18).
    #[inline]
    pub fn get_padded_state(&self, px: usize, py: usize, pz: usize) -> &str {
        let pad_idx = px * 324 + py * 18 + pz;
        let pal_idx = self.padded_voxels[pad_idx] as usize;
        if pal_idx < self.palette.len() {
            &self.palette[pal_idx]
        } else {
            "minecraft:air"
        }
    }
}

/// Mesh generation engine for individual and batch chunk sections.
pub struct SectionMesher;

impl SectionMesher {
    /// Meshes a single `VoxelSection` into a `MeshData` buffer.
    ///
    /// - `section`: The 16x16x16 section with padded 18x18x18 neighbors.
    /// - `culler`: `FaceCuller` engine for sub-microsecond face visibility tests.
    /// - `get_baked_model`: Closure resolving `BakedModel` for a given BlockState string.
    /// - `exclude_hidden_volume`: Whether internal overlapping faces inside complex models are clipped.
    pub fn mesh_section<F>(
        section: &VoxelSection,
        culler: &FaceCuller,
        mut get_baked_model: F,
        _exclude_hidden_volume: bool,
    ) -> MeshData

    where
        F: FnMut(&str) -> Option<Arc<BakedModel>>,
    {
        if section.is_empty {
            return MeshData::new();
        }

        let mut mesh = MeshData::with_capacity(1024, 1536, 512);

        // Pre-resolve palette metadata to avoid re-querying inside 4096 loop
        let palette_metas: Vec<Arc<BlockCullMeta>> = section
            .palette
            .iter()
            .map(|st| culler.get_meta(st, None, None))
            .collect();

        // Palette model cache
        let mut palette_models: Vec<Option<Arc<BakedModel>>> = Vec::with_capacity(section.palette.len());
        for st in &section.palette {
            palette_models.push(get_baked_model(st));
        }

        let world_offset_x = (section.coord.x * 16) as f32;
        let world_offset_y = (section.coord.y * 16) as f32;
        let world_offset_z = (section.coord.z * 16) as f32;

        for lx in 0..16usize {
            let px = lx + 1;
            let wx = world_offset_x + lx as f32;

            for ly in 0..16usize {
                let py = ly + 1;
                let wy = world_offset_y + ly as f32;

                for lz in 0..16usize {
                    let pz = lz + 1;
                    let wz = world_offset_z + lz as f32;

                    let pad_idx = px * 324 + py * 18 + pz;
                    let pal_idx = section.padded_voxels[pad_idx] as usize;
                    if pal_idx >= palette_metas.len() {
                        continue;
                    }

                    let meta = &palette_metas[pal_idx];
                    if meta.is_air {
                        continue;
                    }

                    let block_pos = IVec3::new(wx as i32, wy as i32, wz as i32);
                    let baked_opt = &palette_models[pal_idx];

                    for dir in Direction::ALL {
                        let offset = dir.offset();
                        let npx = (px as i32 + offset.x) as usize;
                        let npy = (py as i32 + offset.y) as usize;
                        let npz = (pz as i32 + offset.z) as usize;

                        let npad_idx = npx * 324 + npy * 18 + npz;
                        let npal_idx = section.padded_voxels[npad_idx] as usize;
                        let n_meta = if npal_idx < palette_metas.len() {
                            Some(&*palette_metas[npal_idx])
                        } else {
                            None
                        };

                        let neighbor_pos = block_pos + offset;

                        // Check visibility
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

                        // Emit face geometry
                        if let Some(baked) = baked_opt {
                            // Emit detailed baked elements faces
                            for el in &baked.elements {
                                if let Some(face) = el.faces.get(&dir) {
                                    emit_baked_face(
                                        &mut mesh,
                                        face,
                                        wx,
                                        wy,
                                        wz,
                                    );
                                }
                            }
                        } else {
                            // Emit canonical unit cube face
                            emit_unit_cube_face(
                                &mut mesh,
                                dir,
                                wx,
                                wy,
                                wz,
                                0, // material slot
                            );
                        }
                    }
                }
            }
        }

        mesh
    }

    /// Meshes a batch of `VoxelSection`s in parallel across multiple CPU cores using Rayon.
    ///
    /// - `sections`: Slice of sections to mesh.
    /// - `culler`: Face culling engine instance.
    /// - `model_provider`: Thread-safe callback providing pre-baked models for blockstates.
    /// - `num_threads`: Thread count (None = conservative default).
    #[cfg(feature = "parallel")]
    pub fn mesh_sections_parallel<F>(
        sections: &[VoxelSection],
        culler: &FaceCuller,
        model_provider: F,
        num_threads: Option<usize>,
    ) -> Result<Vec<(IVec3, MeshData)>, ModelError>
    where
        F: Fn(&str) -> Option<Arc<BakedModel>> + Sync + Send,
    {
        use rayon::prelude::*;

        let available = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2);
        let threads = num_threads.unwrap_or_else(|| (available / 2).clamp(1, 8));

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("mtk-mesher-{}", i))
            .build()
            .map_err(|e| ModelError::ThreadPoolError(e.to_string()))?;

        let results = pool.install(|| {
            sections
                .par_iter()
                .map(|sec| {
                    let local_culler = culler.clone();
                    let mesh = Self::mesh_section(
                        sec,
                        &local_culler,
                        |st| model_provider(st),
                        true,
                    );
                    (sec.coord, mesh)
                })
                .collect()
        });

        Ok(results)
    }
}

/// Helper to emit a single baked face into `MeshData`.
#[inline]
fn emit_baked_face(
    mesh: &mut MeshData,
    face: &BakedFace,
    wx: f32,
    wy: f32,
    wz: f32,
) {
    let base_idx = mesh.positions.len() as u32;
    let n = [face.normal.x, face.normal.y, face.normal.z];

    for i in 0..4 {
        let v = face.vertices[i];
        mesh.positions.push([wx + v.x, wy + v.y, wz + v.z]);
        mesh.normals.push(n);
        mesh.uvs.push([face.uvs[i].x, face.uvs[i].y]);
    }

    mesh.indices.push(base_idx);
    mesh.indices.push(base_idx + 1);
    mesh.indices.push(base_idx + 2);

    mesh.indices.push(base_idx);
    mesh.indices.push(base_idx + 2);
    mesh.indices.push(base_idx + 3);

    mesh.face_materials.push(0);
    mesh.face_tint_indices.push(face.tint_index);
}

/// Helper to emit a canonical unit cube face into `MeshData`.
#[inline]
fn emit_unit_cube_face(
    mesh: &mut MeshData,
    dir: Direction,
    wx: f32,
    wy: f32,
    wz: f32,
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

    mesh.positions.push([wx + v0.x, wy + v0.y, wz + v0.z]);
    mesh.positions.push([wx + v1.x, wy + v1.y, wz + v1.z]);
    mesh.positions.push([wx + v2.x, wy + v2.y, wz + v2.z]);
    mesh.positions.push([wx + v3.x, wy + v3.y, wz + v3.z]);

    for _ in 0..4 {
        mesh.normals.push(n);
    }

    mesh.uvs.push([0.0, 0.0]);
    mesh.uvs.push([0.0, 1.0]);
    mesh.uvs.push([1.0, 1.0]);
    mesh.uvs.push([1.0, 0.0]);

    mesh.indices.push(base_idx);
    mesh.indices.push(base_idx + 1);
    mesh.indices.push(base_idx + 2);

    mesh.indices.push(base_idx);
    mesh.indices.push(base_idx + 2);
    mesh.indices.push(base_idx + 3);

    mesh.face_materials.push(mat_slot);
    mesh.face_tint_indices.push(-1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_section_meshing_solid_cube() {
        let coord = IVec3::new(0, 0, 0);
        let mut core_blocks = vec!["minecraft:stone".to_string(); 4096];
        // Make center block air
        core_blocks[0] = "minecraft:air".to_string();

        let section = VoxelSection::from_flat_array(coord, &core_blocks, |_x, _y, _z| {
            "minecraft:stone".to_string()
        });

        let culler = FaceCuller::default();
        let mesh = SectionMesher::mesh_section(&section, &culler, |_st| None, false);

        // Surrounding neighbors are stone, so only faces adjacent to internal air or boundary with air are rendered
        assert!(mesh.triangle_count() > 0);
    }
}
