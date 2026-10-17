//! # Derived Biome Metadata Finalization
//!
//! Single source of truth for the biome metadata that is **derived** from the
//! section-level 4x4x4 quart biome data ([`SectionBiomes`]):
//!
//! - `biome_column_map`: the surface biome of every block column inside the active bounds.
//! - `primary_biome`: the dominant surface biome, used as a last-resort fallback.
//!
//! Every ingestion path (Anvil save loader, Live Sync baseline assembler, point-cloud
//! reconstruction) must funnel through [`VoxelStorage::finalize_biome_metadata`] instead of
//! re-implementing the derivation locally. That is what keeps save imports and live sync
//! byte-compatible.
//!
//! [`SectionBiomes`]: crate::storage::SectionBiomes

use std::collections::HashMap;

use glam::IVec3;

use super::container::VoxelStorage;

impl VoxelStorage {
    /// Rebuilds `biome_column_map` and `primary_biome` from the section-level quart biomes.
    ///
    /// The surface biome of a column is the biome of the highest section in that column that
    /// carries biome data, sampled at the top quart layer (`qy = 3`). This matches both the
    /// Anvil section layout and the live-sync selection grid, so the same world produces the
    /// same column map regardless of the provider.
    ///
    /// Cost: `O(area * section_height)`; allocation-free per column except the map entries.
    pub fn finalize_biome_metadata(&mut self) {
        let (min_x, min_y, min_z, size_x, size_y, size_z) = self.get_bounds();
        if size_x <= 0 || size_y <= 0 || size_z <= 0 {
            self.biome_column_map.clear();
            self.primary_biome = None;
            return;
        }

        let max_x = min_x + size_x - 1;
        let max_z = min_z + size_z - 1;
        let min_sec_y = min_y >> 4;
        let max_sec_y = (min_y + size_y - 1) >> 4;

        let mut column_map: HashMap<[i32; 2], String> = HashMap::with_capacity(
            (size_x as usize)
                .saturating_mul(size_z as usize)
                .min(1 << 20),
        );
        let mut counts: HashMap<String, u64> = HashMap::new();

        {
            let sections = &self.sections;
            for x in min_x..=max_x {
                let sx = x >> 4;
                let qx = ((x & 15) >> 2) as usize;
                for z in min_z..=max_z {
                    let sz = z >> 4;
                    let qz = ((z & 15) >> 2) as usize;

                    let mut surface: Option<&str> = None;
                    for sy in (min_sec_y..=max_sec_y).rev() {
                        let Some(sec) = sections.get(&IVec3::new(sx, sy, sz)) else {
                            continue;
                        };
                        let Some(biomes) = sec.biomes.as_ref() else {
                            continue;
                        };
                        if biomes.palette.is_empty() {
                            continue;
                        }
                        if let Some(biome) = biomes.get_quart_biome(qx, 3, qz) {
                            surface = Some(biome);
                            break;
                        }
                    }

                    let biome = surface.unwrap_or("minecraft:plains");
                    *counts.entry(biome.to_string()).or_insert(0) += 1;
                    column_map.insert([x, z], biome.to_string());
                }
            }
        }

        // Deterministic dominant biome: highest column count, ties broken by the
        // lexicographically smallest biome id so the result never depends on hash order.
        let primary = counts
            .into_iter()
            .max_by(|(name_a, count_a), (name_b, count_b)| {
                count_a.cmp(count_b).then_with(|| name_b.cmp(name_a))
            })
            .map(|(name, _)| name);

        self.biome_column_map = column_map;
        self.primary_biome = primary;
    }
}
