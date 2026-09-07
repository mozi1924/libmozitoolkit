use std::collections::HashSet;
use std::sync::Arc;

use glam::IVec3;
use mtk_core::mesh::MeshData;
use mtk_cull::FaceCuller;
use mtk_model::baked::BakedModel;

use crate::mesher::SectionMesher;
use crate::types::MesherConfig;
use crate::world::VoxelStorage;

/// Incrementally rebuilds meshes only for dirty sections in a `VoxelStorage`.
pub struct DeltaMesher;

impl DeltaMesher {
    /// Re-meshes all currently dirty sections in `world` and clears their dirty state.
    /// Returns a list of `(section_coord, MeshData)`.
    pub fn rebuild_dirty_sections<F>(
        world: &mut VoxelStorage,
        culler: &FaceCuller,
        mut model_provider: F,
        config: &MesherConfig,
    ) -> Vec<(IVec3, MeshData)>
    where
        F: FnMut(&str) -> Option<Arc<BakedModel>>,
    {
        let dirty_coords: Vec<IVec3> = world.dirty_sections.iter().copied().collect();
        if dirty_coords.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::with_capacity(dirty_coords.len());

        for coord in dirty_coords {
            let padded = world.get_section_padded_array(coord);
            if !padded.is_empty {
                let mesh = SectionMesher::mesh_section(&padded, culler, &mut model_provider, config);
                results.push((coord, mesh));
            }
        }

        world.clear_dirty_sections();
        results
    }

    /// Finds all section coordinates that would be affected by a set of block modifications,
    /// including 3x3x3 neighbor bounds expansion for fluid modifications.
    pub fn find_affected_sections(
        changes: &[(i32, i32, i32)],
        is_fluid_fn: impl Fn(i32, i32, i32) -> bool,
    ) -> HashSet<IVec3> {
        let mut affected = HashSet::new();

        for &(x, y, z) in changes {
            let sec_coord = IVec3::new(x >> 4, y >> 4, z >> 4);
            affected.insert(sec_coord);

            let is_fluid = is_fluid_fn(x, y, z);
            if is_fluid {
                // Expand to 3x3 horizontal window for fluid slope updates
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        for dy in -1..=1 {
                            let nx = x + dx * 16;
                            let ny = y + dy * 16;
                            let nz = z + dz * 16;
                            affected.insert(IVec3::new(nx >> 4, ny >> 4, nz >> 4));
                        }
                    }
                }
            } else {
                // Check 6-direction boundary neighbor sections
                if (x & 15) == 0 {
                    affected.insert(sec_coord + IVec3::new(-1, 0, 0));
                } else if (x & 15) == 15 {
                    affected.insert(sec_coord + IVec3::new(1, 0, 0));
                }
                if (y & 15) == 0 {
                    affected.insert(sec_coord + IVec3::new(0, -1, 0));
                } else if (y & 15) == 15 {
                    affected.insert(sec_coord + IVec3::new(0, 1, 0));
                }
                if (z & 15) == 0 {
                    affected.insert(sec_coord + IVec3::new(0, 0, -1));
                } else if (z & 15) == 15 {
                    affected.insert(sec_coord + IVec3::new(0, 0, 1));
                }
            }
        }

        affected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_mesher_rebuild() {
        let mut world = VoxelStorage::new();
        world.set_bounds(0, 0, 0, 16, 16, 16);
        world.set_block(0, 0, 0, "minecraft:stone", None);

        let culler = FaceCuller::default();
        let config = MesherConfig::default();

        let meshes = DeltaMesher::rebuild_dirty_sections(&mut world, &culler, |_| None, &config);
        assert_eq!(meshes.len(), 1);
        assert!(!meshes[0].1.is_empty());
        assert!(world.dirty_sections.is_empty());
    }
}
