//! # Voxel Point Cloud Storage & Serialization
//!
//! Provides dense, unculled 3D point cloud representations of voxel worlds
//! for lossless persistent storage, scene roundtripping, and host viewport
//! attribute mapping without spatial occlusion culling.

use std::collections::HashMap;

use glam::{IVec3, Vec3};
use mtk_core::attributes::{AttributeData, AttributeDomain, MeshAttribute};
use mtk_core::mesh::MeshData;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::source::VoxelSource;
use crate::storage::biome::SectionBiomes;
use crate::storage::section::SectionStorage;
use crate::storage::world::VoxelStorage;
use crate::types::{MesherConfig, VoxelError};

/// High-throughput flat-buffered representation of a voxel point cloud.
///
/// Designed for zero-copy memoryview sharing across FFI/Python boundaries
/// and direct attribute mapping to Blender/USD point domains without spatial occlusion culling.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoxelPointCloud {
    /// 3D transformed vertex coordinates: `[x0, y0, z0, x1, y1, z1, ...]`.
    /// Computed from block centers `(x + 0.5, y + 0.5, z + 0.5)` under active coordinate system and centering.
    pub positions: Vec<f32>,
    /// Native discrete integer voxel coordinates.
    pub block_x: Vec<i32>,
    pub block_y: Vec<i32>,
    pub block_z: Vec<i32>,
    /// Unique canonical Minecraft blockstate palette.
    pub state_palette: Vec<String>,
    /// Per-point indices into `state_palette`.
    pub state_indices: Vec<i32>,
    /// Unique canonical Minecraft biome palette.
    pub biome_palette: Vec<String>,
    /// Per-point indices into `biome_palette`.
    pub biome_indices: Vec<i32>,
    /// Optional composite light levels (sky in high nibble, block in low nibble).
    pub light_levels: Vec<u8>,
    /// Optional bounding box `[min_x, min_y, min_z, size_x, size_y, size_z]` to preserve origin alignment.
    pub bounds: Option<[i32; 6]>,
}

impl VoxelPointCloud {
    /// Creates an empty `VoxelPointCloud`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty `VoxelPointCloud` with pre-allocated capacity for N points.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(capacity * 3),
            block_x: Vec::with_capacity(capacity),
            block_y: Vec::with_capacity(capacity),
            block_z: Vec::with_capacity(capacity),
            state_palette: Vec::with_capacity(16),
            state_indices: Vec::with_capacity(capacity),
            biome_palette: Vec::with_capacity(16),
            biome_indices: Vec::with_capacity(capacity),
            light_levels: Vec::with_capacity(capacity),
            bounds: None,
        }
    }

    /// Returns the total number of voxel points stored in this cloud.
    #[inline]
    pub fn len(&self) -> usize {
        self.block_x.len()
    }

    /// Checks if the point cloud contains no voxel points.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.block_x.is_empty()
    }

    /// Appends a single voxel point into the flat arrays.
    pub fn push(
        &mut self,
        pos: [f32; 3],
        block_pos: [i32; 3],
        state: String,
        biome: String,
        light: u8,
    ) {
        self.positions.extend_from_slice(&pos);
        self.block_x.push(block_pos[0]);
        self.block_y.push(block_pos[1]);
        self.block_z.push(block_pos[2]);
        let state_idx = self.intern_state(&state);
        self.state_indices.push(state_idx);
        let biome_idx = self.intern_biome(&biome);
        self.biome_indices.push(biome_idx);
        self.light_levels.push(light);
    }

    /// Interns a blockstate into the palette and returns its index.
    pub fn intern_state(&mut self, state: &str) -> i32 {
        intern_palette(&mut self.state_palette, state)
    }

    /// Interns a biome into the palette and returns its index.
    pub fn intern_biome(&mut self, biome: &str) -> i32 {
        intern_palette(&mut self.biome_palette, biome)
    }

    /// Resolves the blockstate string for point `index`.
    #[inline]
    pub fn state_at(&self, index: usize) -> Option<&str> {
        self.state_indices
            .get(index)
            .and_then(|&idx| self.state_palette.get(idx.max(0) as usize))
            .map(String::as_str)
    }

    /// Resolves the biome string for point `index`.
    #[inline]
    pub fn biome_at(&self, index: usize) -> Option<&str> {
        self.biome_indices
            .get(index)
            .and_then(|&idx| self.biome_palette.get(idx.max(0) as usize))
            .map(String::as_str)
    }

    /// Efficient batch constructor from palette indices, avoiding allocating millions of strings on the host.
    pub fn from_indexed_arrays(
        positions: Vec<f32>,
        block_x: Vec<i32>,
        block_y: Vec<i32>,
        block_z: Vec<i32>,
        state_indices: &[i32],
        state_palette: &[String],
        biome_indices: Option<&[i32]>,
        biome_palette: Option<&[String]>,
        light_levels: Option<Vec<u8>>,
        bounds: Option<[i32; 6]>,
    ) -> Result<Self, VoxelError> {
        let count = block_x.len();
        if positions.len() != count * 3
            || block_y.len() != count
            || block_z.len() != count
            || state_indices.len() != count
        {
            return Err(VoxelError::InvalidPointCloud(format!(
                "Array lengths mismatch: positions={}, block_x={}, block_y={}, block_z={}, state_indices={}",
                positions.len(),
                block_x.len(),
                block_y.len(),
                block_z.len(),
                state_indices.len()
            )));
        }
        validate_palette_indices("state", state_indices, state_palette)?;

        let (biome_palette, biome_indices) = match (biome_indices, biome_palette) {
            (Some(indices), Some(palette)) => {
                if indices.len() != count {
                    return Err(VoxelError::InvalidPointCloud(format!(
                        "biome_indices length ({}) must match point count ({})",
                        indices.len(),
                        count
                    )));
                }
                validate_palette_indices("biome", indices, palette)?;
                (palette.to_vec(), indices.to_vec())
            }
            (Some(_), None) => {
                return Err(VoxelError::InvalidPointCloud(
                    "biome_indices requires a biome_palette".to_string(),
                ));
            }
            _ => (vec!["minecraft:plains".to_string()], vec![0; count]),
        };

        Ok(Self {
            positions,
            block_x,
            block_y,
            block_z,
            state_palette: state_palette.to_vec(),
            state_indices: state_indices.to_vec(),
            biome_palette,
            biome_indices,
            light_levels: light_levels.unwrap_or_else(|| vec![0u8; count]),
            bounds,
        })
    }

    /// Converts this `VoxelPointCloud` into a pure-point `MeshData` with standard
    /// attributes attached on the `Point` domain (without edges or polygons).
    pub fn to_mesh_data(&self) -> MeshData {
        let count = self.len();
        let mut mesh = MeshData::with_capacity(count, 0, 0);

        for i in 0..count {
            let p_off = i * 3;
            mesh.positions.push([
                self.positions[p_off],
                self.positions[p_off + 1],
                self.positions[p_off + 2],
            ]);
        }

        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_x",
            AttributeDomain::Point,
            AttributeData::Int32(self.block_x.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_y",
            AttributeDomain::Point,
            AttributeData::Int32(self.block_y.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_block_z",
            AttributeDomain::Point,
            AttributeData::Int32(self.block_z.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_state_idx",
            AttributeDomain::Point,
            AttributeData::Int32(self.state_indices.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_palette",
            AttributeDomain::Mesh,
            AttributeData::String(self.state_palette.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_idx",
            AttributeDomain::Point,
            AttributeData::Int32(self.biome_indices.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome_palette",
            AttributeDomain::Mesh,
            AttributeData::String(self.biome_palette.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_light",
            AttributeDomain::Point,
            AttributeData::UInt8(self.light_levels.clone()),
        ));

        if let Some(b) = self.bounds {
            mesh.add_custom_attribute(MeshAttribute::new(
                "mtk_bounds",
                AttributeDomain::Mesh,
                AttributeData::Int32(b.to_vec()),
            ));
        }

        mesh
    }

    /// Reconstructs a `VoxelPointCloud` from a `MeshData` containing Point-domain attributes.
    pub fn from_mesh_data(mesh: &MeshData) -> Result<Self, VoxelError> {
        let count = mesh.positions.len();
        if count == 0 {
            return Ok(Self::new());
        }

        let mut positions = Vec::with_capacity(count * 3);
        for p in &mesh.positions {
            positions.extend_from_slice(p);
        }

        let block_x = mesh
            .get_custom_attribute("mtk_block_x")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_x attribute".to_string(),
                )
            })?;

        let block_y = mesh
            .get_custom_attribute("mtk_block_y")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_y attribute".to_string(),
                )
            })?;

        let block_z = mesh
            .get_custom_attribute("mtk_block_z")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_z attribute".to_string(),
                )
            })?;

        let state_indices = mesh
            .get_custom_attribute("mtk_state_idx")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_state_idx attribute".to_string(),
                )
            })?;

        let state_palette = mesh
            .get_custom_attribute("mtk_palette")
            .and_then(|a| match &a.data {
                AttributeData::String(v) if !v.is_empty() => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_palette attribute".to_string(),
                )
            })?;
        validate_palette_indices("state", &state_indices, &state_palette)?;

        let biome_indices = mesh
            .get_custom_attribute("mtk_biome_idx")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec![0; count]);
        let biome_palette = mesh
            .get_custom_attribute("mtk_biome_palette")
            .and_then(|a| match &a.data {
                AttributeData::String(v) if !v.is_empty() => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec!["minecraft:plains".to_string()]);
        validate_palette_indices("biome", &biome_indices, &biome_palette)?;

        let light_levels = mesh
            .get_custom_attribute("mtk_light")
            .and_then(|a| match &a.data {
                AttributeData::UInt8(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec![0u8; count]);

        let bounds = mesh
            .get_custom_attribute("mtk_bounds")
            .and_then(|a| match &a.data {
                AttributeData::Int32(v) if v.len() == 6 => {
                    Some([v[0], v[1], v[2], v[3], v[4], v[5]])
                }
                _ => None,
            });

        Ok(Self {
            positions,
            block_x,
            block_y,
            block_z,
            state_palette,
            state_indices,
            biome_palette,
            biome_indices,
            light_levels,
            bounds,
        })
    }

    /// Extracts an unculled `VoxelPointCloud` from `VoxelStorage`.
    ///
    /// Every non-air block in non-empty sections is preserved as an authoritative 3D point.
    /// No face culling or hidden volume exclusion is performed.
    pub fn extract_from_storage(storage: &VoxelStorage, config: &MesherConfig) -> Self {
        let non_empty = storage.get_all_non_empty_sections();
        if non_empty.is_empty() {
            return Self::new();
        }

        let mut effective_config = config.clone();
        let (bx, by, bz, sx, sy, sz) = storage.get_bounds();
        let has_bounds = sx > 0 && sy > 0 && sz > 0;
        if effective_config.origin_centered
            && effective_config.selection_bounds.is_none()
            && has_bounds
        {
            effective_config.selection_bounds = Some(([bx, by, bz], [sx, sy, sz]));
        }

        // Pre-estimate non-air block count for single reallocation
        let total_est: usize = non_empty
            .iter()
            .filter_map(|c| storage.sections.get(c))
            .map(|sec| sec.non_air_count as usize)
            .sum();

        let mut cloud = Self::with_capacity(total_est);
        if has_bounds {
            cloud.bounds = Some([bx, by, bz, sx, sy, sz]);
        }
        let mut state_map = HashMap::new();
        let mut biome_map = HashMap::new();

        for sec_coord in non_empty {
            if let Some(sec) = storage.sections.get(&sec_coord) {
                if sec.is_empty() {
                    continue;
                }
                let base_x = sec_coord.x * 16;
                let base_y = sec_coord.y * 16;
                let base_z = sec_coord.z * 16;

                let (is_fully_inside, is_fully_outside) =
                    if storage.has_explicit_bounds && has_bounds {
                        let b_min_x = bx;
                        let b_max_x = bx + sx - 1;
                        let b_min_y = by;
                        let b_max_y = by + sy - 1;
                        let b_min_z = bz;
                        let b_max_z = bz + sz - 1;

                        let sec_min_x = base_x;
                        let sec_max_x = base_x + 15;
                        let sec_min_y = base_y;
                        let sec_max_y = base_y + 15;
                        let sec_min_z = base_z;
                        let sec_max_z = base_z + 15;

                        let outside = sec_max_x < b_min_x
                            || sec_min_x > b_max_x
                            || sec_max_y < b_min_y
                            || sec_min_y > b_max_y
                            || sec_max_z < b_min_z
                            || sec_min_z > b_max_z;

                        let inside = sec_min_x >= b_min_x
                            && sec_max_x <= b_max_x
                            && sec_min_y >= b_min_y
                            && sec_max_y <= b_max_y
                            && sec_min_z >= b_min_z
                            && sec_max_z <= b_max_z;

                        (inside, outside)
                    } else {
                        (true, false)
                    };

                if is_fully_outside {
                    continue;
                }

                for lx in 0..16 {
                    for ly in 0..16 {
                        for lz in 0..16 {
                            let state = sec.get_local_state(lx, ly, lz);
                            if state.is_empty() || state == "minecraft:air" {
                                continue;
                            }

                            let wx = base_x + lx as i32;
                            let wy = base_y + ly as i32;
                            let wz = base_z + lz as i32;

                            if !is_fully_inside && !storage.contains(wx, wy, wz) {
                                continue;
                            }

                            // Compute center point under active coordinate transform and centering
                            let center =
                                Vec3::new(wx as f32 + 0.5, wy as f32 + 0.5, wz as f32 + 0.5);
                            let transformed = effective_config.transform_position(center);

                            let biome = storage.get_biome(wx, wy, wz);

                            cloud.positions.extend_from_slice(&[
                                transformed.x,
                                transformed.y,
                                transformed.z,
                            ]);
                            cloud.block_x.push(wx);
                            cloud.block_y.push(wy);
                            cloud.block_z.push(wz);
                            cloud.state_indices.push(palette_index(
                                &mut cloud.state_palette,
                                &mut state_map,
                                state,
                            ));
                            cloud.biome_indices.push(palette_index(
                                &mut cloud.biome_palette,
                                &mut biome_map,
                                biome,
                            ));
                            cloud.light_levels.push(0);
                        }
                    }
                }
            }
        }

        cloud
    }

    /// Reconstructs a full `VoxelStorage` instance from this point cloud.
    ///
    /// Computes discrete section placements, updates palettes, calculates bounding box,
    /// and marks all populated sections as dirty for subsequent meshing.
    /// Employs fast section grouping and single-allocation primary biome heuristics
    /// to avoid multimillion-entry hash table allocations.
    pub fn reconstruct_storage(&self) -> VoxelStorage {
        let mut storage = VoxelStorage::new();
        let count = self.len();
        if count == 0 {
            return storage;
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut min_z = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut max_z = i32::MIN;

        for i in 0..count {
            let bx = self.block_x[i];
            let by = self.block_y[i];
            let bz = self.block_z[i];
            let state = self.state_at(i).unwrap_or("minecraft:air");

            min_x = min_x.min(bx);
            min_y = min_y.min(by);
            min_z = min_z.min(bz);
            max_x = max_x.max(bx);
            max_y = max_y.max(by);
            max_z = max_z.max(bz);

            let sec_coord = IVec3::new(bx >> 4, by >> 4, bz >> 4);
            let lx = (bx & 15) as usize;
            let ly = (by & 15) as usize;
            let lz = (bz & 15) as usize;

            let sec = storage
                .sections
                .entry(sec_coord)
                .or_insert_with(|| SectionStorage::new(sec_coord));
            sec.set_local(lx, ly, lz, state);

            // Rebuild the section-level 4x4x4 quart biomes from the per-point data so the
            // reconstructed storage is structurally identical to save/live-sync storage.
            if let Some(biome) = self.biome_at(i) {
                if !biome.is_empty() {
                    let sb = sec
                        .biomes
                        .get_or_insert_with(|| SectionBiomes::blank(vec![biome.to_string()]));
                    sb.set_sampled_quart_biome((lx >> 2) & 3, (ly >> 2) & 3, (lz >> 2) & 3, biome);
                }
            }
        }

        // Fill quart cells that carry no point with the nearest sampled biome, then derive
        // the shared column map / primary biome through the single authoritative path.
        for sec in storage.sections.values_mut() {
            if let Some(biomes) = sec.biomes.as_mut() {
                biomes.fill_unwritten_nearest();
            }
        }

        // If bounds were recorded on the cloud, preserve exact bounds for origin stability.
        if let Some([bx, by, bz, sx, sy, sz]) = self.bounds {
            storage.set_bounds(bx, by, bz, sx, sy, sz);
        } else if min_x <= max_x && min_y <= max_y && min_z <= max_z {
            storage.set_bounds(
                min_x,
                min_y,
                min_z,
                max_x - min_x + 1,
                max_y - min_y + 1,
                max_z - min_z + 1,
            );
        }

        storage.finalize_biome_metadata();
        storage.mark_all_sections_dirty();
        storage
    }

    /// Creates an abstract `VoxelSource` wrapping this point cloud.
    pub fn as_source(&self) -> PointCloudVoxelSource {
        PointCloudVoxelSource::from_point_cloud(self)
    }
}

fn intern_palette(palette: &mut Vec<String>, value: &str) -> i32 {
    if let Some(idx) = palette.iter().position(|entry| entry == value) {
        return idx as i32;
    }
    let idx = palette.len() as i32;
    palette.push(value.to_string());
    idx
}

fn palette_index(palette: &mut Vec<String>, map: &mut HashMap<String, i32>, value: &str) -> i32 {
    if let Some(&idx) = map.get(value) {
        return idx;
    }
    let idx = palette.len() as i32;
    palette.push(value.to_string());
    map.insert(value.to_string(), idx);
    idx
}

fn validate_palette_indices(
    label: &str,
    indices: &[i32],
    palette: &[String],
) -> Result<(), VoxelError> {
    if indices.is_empty() {
        return Ok(());
    }
    if palette.is_empty() {
        return Err(VoxelError::InvalidPointCloud(format!(
            "{label} indices reference an empty {label} palette"
        )));
    }
    if let Some(&bad) = indices
        .iter()
        .find(|&&idx| idx < 0 || idx as usize >= palette.len())
    {
        return Err(VoxelError::InvalidPointCloud(format!(
            "{label} index {bad} is out of bounds for {label} palette length {}",
            palette.len()
        )));
    }
    Ok(())
}

/// Abstract `VoxelSource` implementation backed by a pre-parsed `VoxelPointCloud`.
pub struct PointCloudVoxelSource {
    sections: HashMap<IVec3, SectionStorage>,
    biomes: HashMap<IVec3, String>,
    bounds: Option<(IVec3, IVec3)>,
}

impl PointCloudVoxelSource {
    /// Builds a `PointCloudVoxelSource` by partitioning point cloud points into chunk sections.
    pub fn from_point_cloud(cloud: &VoxelPointCloud) -> Self {
        let mut sections: HashMap<IVec3, SectionStorage> = HashMap::new();
        let mut biomes: HashMap<IVec3, String> = HashMap::new();

        let count = cloud.len();
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut min_z = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut max_z = i32::MIN;

        for i in 0..count {
            let bx = cloud.block_x[i];
            let by = cloud.block_y[i];
            let bz = cloud.block_z[i];
            let state = cloud.state_at(i).unwrap_or("minecraft:air");

            let sec_coord = IVec3::new(bx.div_euclid(16), by.div_euclid(16), bz.div_euclid(16));
            let lx = bx.rem_euclid(16) as usize;
            let ly = by.rem_euclid(16) as usize;
            let lz = bz.rem_euclid(16) as usize;

            let sec = sections
                .entry(sec_coord)
                .or_insert_with(|| SectionStorage::new(sec_coord));
            sec.set_local(lx, ly, lz, state);

            if let Some(biome) = cloud.biome_at(i) {
                biomes.insert(IVec3::new(bx, by, bz), biome.to_string());
            }

            min_x = min_x.min(bx);
            min_y = min_y.min(by);
            min_z = min_z.min(bz);
            max_x = max_x.max(bx);
            max_y = max_y.max(by);
            max_z = max_z.max(bz);
        }

        let bounds = if min_x <= max_x {
            Some((
                IVec3::new(
                    min_x.div_euclid(16),
                    min_y.div_euclid(16),
                    min_z.div_euclid(16),
                ),
                IVec3::new(
                    max_x.div_euclid(16),
                    max_y.div_euclid(16),
                    max_z.div_euclid(16),
                ),
            ))
        } else {
            None
        };

        Self {
            sections,
            biomes,
            bounds,
        }
    }
}

impl VoxelSource for PointCloudVoxelSource {
    fn source_name(&self) -> &str {
        "PointCloudVoxelSource"
    }

    fn estimated_section_count(&self) -> Option<usize> {
        Some(self.sections.len())
    }

    fn has_section(&self, sx: i32, sy: i32, sz: i32) -> bool {
        self.sections.contains_key(&IVec3::new(sx, sy, sz))
    }

    fn load_section(
        &mut self,
        sx: i32,
        sy: i32,
        sz: i32,
    ) -> Result<Option<SectionStorage>, VoxelError> {
        let coord = IVec3::new(sx, sy, sz);
        Ok(self.sections.remove(&coord))
    }

    fn section_bounds(&self) -> Option<(IVec3, IVec3)> {
        self.bounds
    }

    fn sample_biome(&self, x: i32, y: i32, z: i32) -> Option<&str> {
        self.biomes.get(&IVec3::new(x, y, z)).map(|s| s.as_str())
    }
}
