//! `PointCloudVoxelSource` abstract VoxelSource backed by a `VoxelPointCloud`.

use std::collections::HashMap;

use glam::IVec3;

use crate::source::VoxelSource;
use crate::storage::section::SectionStorage;
use crate::types::VoxelError;

use super::VoxelPointCloud;

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

        let has_entities = !cloud.entity_palette.is_empty() && cloud.entity_indices.len() == count;

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

            if has_entities {
                let ent_idx = cloud.entity_indices[i];
                if ent_idx >= 0 {
                    if let Some(json_str) = cloud.entity_palette.get(ent_idx as usize) {
                        if let Some(entity_data) =
                            mtk_model::BlockEntityData::from_visual_json(json_str)
                        {
                            let local_idx = crate::storage::block_index(lx, ly, lz) as u16;
                            sec.set_block_entity(local_idx, Some(entity_data));
                        }
                    }
                }
            }

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
