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
    /// Canonical Minecraft blockstate string per point.
    pub block_states: Vec<String>,
    /// Canonical Minecraft biome registry string per point.
    pub biomes: Vec<String>,
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
            block_states: Vec::with_capacity(capacity),
            biomes: Vec::with_capacity(capacity),
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
        self.block_states.push(state);
        self.biomes.push(biome);
        self.light_levels.push(light);
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
            "mtk_block_state",
            AttributeDomain::Point,
            AttributeData::String(self.block_states.clone()),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_biome",
            AttributeDomain::Point,
            AttributeData::String(self.biomes.clone()),
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

        let block_states = mesh
            .get_custom_attribute("mtk_block_state")
            .and_then(|a| match &a.data {
                AttributeData::String(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                VoxelError::InvalidPointCloud(
                    "Missing or malformed mtk_block_state attribute".to_string(),
                )
            })?;

        let biomes = mesh
            .get_custom_attribute("mtk_biome")
            .and_then(|a| match &a.data {
                AttributeData::String(v) if v.len() == count => Some(v.clone()),
                _ => None,
            })
            .unwrap_or_else(|| vec!["minecraft:plains".to_string(); count]);

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
            block_states,
            biomes,
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

        for sec_coord in non_empty {
            if let Some(sec) = storage.sections.get(&sec_coord) {
                if sec.is_empty() {
                    continue;
                }
                let base_x = sec_coord.x * 16;
                let base_y = sec_coord.y * 16;
                let base_z = sec_coord.z * 16;

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

                            // Compute center point under active coordinate transform and centering
                            let center =
                                Vec3::new(wx as f32 + 0.5, wy as f32 + 0.5, wz as f32 + 0.5);
                            let transformed = effective_config.transform_position(center);

                            let biome = storage.get_biome(wx, wy, wz);

                            cloud.push(
                                [transformed.x, transformed.y, transformed.z],
                                [wx, wy, wz],
                                state.to_string(),
                                biome.to_string(),
                                0,
                            );
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
            let state = &self.block_states[i];
            let biome = if i < self.biomes.len() {
                Some(self.biomes[i].as_str())
            } else {
                None
            };

            storage.set_block(bx, by, bz, state, biome);

            min_x = min_x.min(bx);
            min_y = min_y.min(by);
            min_z = min_z.min(bz);
            max_x = max_x.max(bx);
            max_y = max_y.max(by);
            max_z = max_z.max(bz);
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

        storage.mark_all_sections_dirty();
        storage
    }

    /// Creates an abstract `VoxelSource` wrapping this point cloud.
    pub fn as_source(&self) -> PointCloudVoxelSource {
        PointCloudVoxelSource::from_point_cloud(self)
    }
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
            let state = &cloud.block_states[i];

            let sec_coord = IVec3::new(bx.div_euclid(16), by.div_euclid(16), bz.div_euclid(16));
            let lx = bx.rem_euclid(16) as usize;
            let ly = by.rem_euclid(16) as usize;
            let lz = bz.rem_euclid(16) as usize;

            let sec = sections
                .entry(sec_coord)
                .or_insert_with(|| SectionStorage::new(sec_coord));
            sec.set_local(lx, ly, lz, state);

            if i < cloud.biomes.len() {
                biomes.insert(IVec3::new(bx, by, bz), cloud.biomes[i].clone());
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
