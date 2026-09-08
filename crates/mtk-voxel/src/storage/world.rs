use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{HashMap, HashSet};

use glam::IVec3;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::crc::extract_canonical_state_str;
use crate::storage::{PaddedVoxelArray, SectionStorage};

#[cfg(feature = "serde")]
mod serde_atomic_u64 {
    use super::*;

    pub fn serialize<S>(val: &AtomicU64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(val.load(Ordering::Relaxed))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<AtomicU64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = u64::deserialize(deserializer)?;
        Ok(AtomicU64::new(v))
    }
}

/// 3D sparse world voxel container managing multiple 16x16x16 chunk sections,
/// delta synchronization, and boundary dirty tracking.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoxelStorage {
    /// Minimum world bounding coordinate (X).
    pub min_x: i32,
    /// Minimum world bounding coordinate (Y).
    pub min_y: i32,
    /// Minimum world bounding coordinate (Z).
    pub min_z: i32,
    /// Selection box size along X axis.
    pub size_x: i32,
    /// Selection box size along Y axis.
    pub size_y: i32,
    /// Selection box size along Z axis.
    pub size_z: i32,
    /// Sparse map of section coordinates `(sx, sy, sz)` to `SectionStorage`.
    pub sections: HashMap<IVec3, SectionStorage>,
    /// Global biome map by block coordinate `(x, y, z)`.
    pub biome_map: HashMap<IVec3, String>,
    /// Default primary biome when unspecified.
    pub primary_biome: Option<String>,
    /// Set of section coordinates that have been modified and require remeshing.
    pub dirty_sections: HashSet<IVec3>,
    /// Set of section coordinates known to be completely empty.
    pub known_empty_sections: HashSet<IVec3>,
    /// Incremental generation counter (Atomic for lock-free multi-thread sync).
    #[cfg_attr(feature = "serde", serde(with = "serde_atomic_u64"))]
    pub generation: AtomicU64,
}

impl Clone for VoxelStorage {
    fn clone(&self) -> Self {
        Self {
            min_x: self.min_x,
            min_y: self.min_y,
            min_z: self.min_z,
            size_x: self.size_x,
            size_y: self.size_y,
            size_z: self.size_z,
            sections: self.sections.clone(),
            biome_map: self.biome_map.clone(),
            primary_biome: self.primary_biome.clone(),
            dirty_sections: self.dirty_sections.clone(),
            known_empty_sections: self.known_empty_sections.clone(),
            generation: AtomicU64::new(self.generation.load(Ordering::Relaxed)),
        }
    }
}

impl Default for VoxelStorage {
    fn default() -> Self {
        Self {
            min_x: 0,
            min_y: 0,
            min_z: 0,
            size_x: 0,
            size_y: 0,
            size_z: 0,
            sections: HashMap::new(),
            biome_map: HashMap::new(),
            primary_biome: None,
            dirty_sections: HashSet::new(),
            known_empty_sections: HashSet::new(),
            generation: AtomicU64::new(0),
        }
    }
}

impl VoxelStorage {
    /// Creates a new empty `VoxelStorage`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets current generation counter value atomically.
    #[inline]
    pub fn get_generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Advances the generation counter atomically and returns new value.
    #[inline]
    pub fn advance_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Sets the generation counter value atomically.
    #[inline]
    pub fn set_generation(&self, val: u64) {
        self.generation.store(val, Ordering::Release);
    }

    /// Clears all sections, biomes, and bounds.
    pub fn clear(&mut self) {
        self.sections.clear();
        self.biome_map.clear();
        self.dirty_sections.clear();
        self.known_empty_sections.clear();
        self.primary_biome = None;
        self.min_x = 0;
        self.min_y = 0;
        self.min_z = 0;
        self.size_x = 0;
        self.size_y = 0;
        self.size_z = 0;
        self.advance_generation();
    }

    /// Checks if world coordinate `(x, y, z)` falls within the active selection bounds.
    #[inline]
    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        if self.size_x <= 0 || self.size_y <= 0 || self.size_z <= 0 {
            return false;
        }
        x >= self.min_x
            && x < self.min_x + self.size_x
            && y >= self.min_y
            && y < self.min_y + self.size_y
            && z >= self.min_z
            && z < self.min_z + self.size_z
    }

    /// Updates selection bounding box with incremental pruning and boundary seam dirtying.
    pub fn set_bounds(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
    ) -> bool {
        let old_bounds = (
            self.min_x, self.min_y, self.min_z, self.size_x, self.size_y, self.size_z,
        );
        let new_bounds = (min_x, min_y, min_z, size_x, size_y, size_z);
        if old_bounds == new_bounds {
            return false;
        }

        let old_has_data = self.size_x > 0 && self.size_y > 0 && self.size_z > 0 && !self.sections.is_empty();

        if !old_has_data || size_x <= 0 || size_y <= 0 || size_z <= 0 {
            if self.size_x > 0 {
                self.clear();
            }
            self.min_x = min_x;
            self.min_y = min_y;
            self.min_z = min_z;
            self.size_x = size_x;
            self.size_y = size_y;
            self.size_z = size_z;
            self.advance_generation();
            return true;
        }

        // Calculate 3D overlap between old and new bounds
        let inter_min_x = self.min_x.max(min_x);
        let inter_max_x = (self.min_x + self.size_x - 1).min(min_x + size_x - 1);
        let inter_min_y = self.min_y.max(min_y);
        let inter_max_y = (self.min_y + self.size_y - 1).min(min_y + size_y - 1);
        let inter_min_z = self.min_z.max(min_z);
        let inter_max_z = (self.min_z + self.size_z - 1).min(min_z + size_z - 1);

        let has_overlap =
            inter_min_x <= inter_max_x && inter_min_y <= inter_max_y && inter_min_z <= inter_max_z;

        if !has_overlap {
            self.clear();
            self.min_x = min_x;
            self.min_y = min_y;
            self.min_z = min_z;
            self.size_x = size_x;
            self.size_y = size_y;
            self.size_z = size_z;
            self.advance_generation();
            return true;
        }

        let new_max_x = min_x + size_x - 1;
        let new_max_y = min_y + size_y - 1;
        let new_max_z = min_z + size_z - 1;

        let new_min_sec_x = min_x >> 4;
        let new_max_sec_x = new_max_x >> 4;
        let new_min_sec_y = min_y >> 4;
        let new_max_sec_y = new_max_y >> 4;
        let new_min_sec_z = min_z >> 4;
        let new_max_sec_z = new_max_z >> 4;

        // Prune out-of-bounds sections
        self.sections.retain(|coord, _| {
            coord.x >= new_min_sec_x
                && coord.x <= new_max_sec_x
                && coord.y >= new_min_sec_y
                && coord.y <= new_max_sec_y
                && coord.z >= new_min_sec_z
                && coord.z <= new_max_sec_z
        });

        // Prune out-of-bounds biomes
        self.biome_map.retain(|pos, _| {
            pos.x >= min_x
                && pos.x <= new_max_x
                && pos.y >= min_y
                && pos.y <= new_max_y
                && pos.z >= min_z
                && pos.z <= new_max_z
        });

        self.min_x = min_x;
        self.min_y = min_y;
        self.min_z = min_z;
        self.size_x = size_x;
        self.size_y = size_y;
        self.size_z = size_z;
        self.advance_generation();

        // Mark boundary seam sections dirty
        for coord in self.sections.keys() {
            let is_boundary = coord.x == new_min_sec_x
                || coord.x == new_max_sec_x
                || coord.y == new_min_sec_y
                || coord.y == new_max_sec_y
                || coord.z == new_min_sec_z
                || coord.z == new_max_sec_z;
            if is_boundary {
                self.dirty_sections.insert(*coord);
            }
        }

        true
    }

    /// Gets the blockstate string at world coordinate `(x, y, z)`.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> &str {
        let sec_coord = IVec3::new(x >> 4, y >> 4, z >> 4);
        if let Some(sec) = self.sections.get(&sec_coord) {
            let lx = (x & 15) as usize;
            let ly = (y & 15) as usize;
            let lz = (z & 15) as usize;
            sec.get_local_state(lx, ly, lz)
        } else {
            "minecraft:air"
        }
    }

    /// Sets the blockstate at world coordinate `(x, y, z)` and marks dirty sections.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, state: &str, biome: Option<&str>) -> bool {
        let sec_coord = IVec3::new(x >> 4, y >> 4, z >> 4);
        let lx = (x & 15) as usize;
        let ly = (y & 15) as usize;
        let lz = (z & 15) as usize;

        if let Some(b) = biome {
            self.biome_map.insert(IVec3::new(x, y, z), b.to_string());
        }

        let sec = self
            .sections
            .entry(sec_coord)
            .or_insert_with(|| SectionStorage::new(sec_coord));

        let changed = sec.set_local(lx, ly, lz, state);
        if changed {
            self.dirty_sections.insert(sec_coord);
            self.known_empty_sections.remove(&sec_coord);

            let has_bounds = self.size_x > 0 && self.size_y > 0 && self.size_z > 0;
            let max_x = self.min_x + self.size_x - 1;
            let max_y = self.min_y + self.size_y - 1;
            let max_z = self.min_z + self.size_z - 1;

            // Check boundary neighbors and mark dirty for cross-section face culling
            if (x & 15) == 0 && (!has_bounds || x > self.min_x) {
                self.dirty_sections.insert(sec_coord + IVec3::new(-1, 0, 0));
            } else if (x & 15) == 15 && (!has_bounds || x < max_x) {
                self.dirty_sections.insert(sec_coord + IVec3::new(1, 0, 0));
            }
            if (y & 15) == 0 && (!has_bounds || y > self.min_y) {
                self.dirty_sections.insert(sec_coord + IVec3::new(0, -1, 0));
            } else if (y & 15) == 15 && (!has_bounds || y < max_y) {
                self.dirty_sections.insert(sec_coord + IVec3::new(0, 1, 0));
            }
            if (z & 15) == 0 && (!has_bounds || z > self.min_z) {
                self.dirty_sections.insert(sec_coord + IVec3::new(0, 0, -1));
            } else if (z & 15) == 15 && (!has_bounds || z < max_z) {
                self.dirty_sections.insert(sec_coord + IVec3::new(0, 0, 1));
            }

            self.advance_generation();
        }

        changed
    }

    /// Applies a batch of block delta modifications. Returns a vector of effective changes `(x, y, z, old_state, new_state)`.
    pub fn apply_delta_update(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        changes: &[(i32, i32, i32, &str)],
    ) -> Vec<(i32, i32, i32, String, String)> {
        if self.size_x > 0
            && self.size_y > 0
            && self.size_z > 0
            && (self.min_x != min_x || self.min_y != min_y || self.min_z != min_z)
        {
            return Vec::new();
        }

        let mut applied = Vec::new();

        for &(x, y, z, new_state) in changes {
            let old_state = self.get_block(x, y, z).to_string();
            let new_canon = extract_canonical_state_str(new_state);
            if old_state != new_canon {
                self.set_block(x, y, z, new_canon, None);
                applied.push((x, y, z, old_state, new_canon.to_string()));
            }
        }

        applied
    }

    /// Ingests a full binary/snapshot packet with block palette and optional biome stream.
    pub fn set_full_snapshot(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: &[String],
        grid_indices: &[u16],
        biome_palette: Option<&[String]>,
        biome_indices: Option<&[u16]>,
    ) {
        self.clear();
        self.min_x = min_x;
        self.min_y = min_y;
        self.min_z = min_z;
        self.size_x = size_x;
        self.size_y = size_y;
        self.size_z = size_z;

        let total = (size_x * size_y * size_z) as usize;
        let p_len = palette.len();
        let has_biomes = biome_palette.is_some() && biome_indices.is_some();

        if let Some(bp) = biome_palette {
            if let Some(first) = bp.first() {
                self.primary_biome = Some(first.clone());
            }
        }

        for idx in 0..total.min(grid_indices.len()) {
            let p_idx = grid_indices[idx] as usize;
            if p_idx >= p_len {
                continue;
            }
            let state = &palette[p_idx];

            let rem = idx % (size_y * size_z) as usize;
            let lx = idx / (size_y * size_z) as usize;
            let ly = rem / size_z as usize;
            let lz = rem % size_z as usize;

            let wx = min_x + lx as i32;
            let wy = min_y + ly as i32;
            let wz = min_z + lz as i32;

            let sec_coord = IVec3::new(wx >> 4, wy >> 4, wz >> 4);
            let sec = self
                .sections
                .entry(sec_coord)
                .or_insert_with(|| SectionStorage::new(sec_coord));

            let blx = (wx & 15) as usize;
            let bly = (wy & 15) as usize;
            let blz = (wz & 15) as usize;

            sec.set_local(blx, bly, blz, state);

            if has_biomes {
                if let (Some(bp), Some(bi)) = (biome_palette, biome_indices) {
                    if idx < bi.len() {
                        let b_idx = bi[idx] as usize;
                        if b_idx < bp.len() {
                            self.biome_map.insert(IVec3::new(wx, wy, wz), bp[b_idx].clone());
                        }
                    }
                }
            }
        }

        self.mark_all_sections_dirty();
    }

    /// Returns the biome registry identifier for block coordinate `(x, y, z)`.
    pub fn get_biome(&self, x: i32, y: i32, z: i32) -> &str {
        if let Some(b) = self.biome_map.get(&IVec3::new(x, y, z)) {
            return b;
        }
        if let Some(ref pb) = self.primary_biome {
            return pb;
        }
        "minecraft:plains"
    }

    /// Marks all existing sections dirty for a full rebuild.
    pub fn mark_all_sections_dirty(&mut self) {
        self.dirty_sections.clear();
        for coord in self.sections.keys() {
            self.dirty_sections.insert(*coord);
        }
    }

    /// Clears the dirty sections set after meshing.
    pub fn clear_dirty_sections(&mut self) {
        self.dirty_sections.clear();
    }

    /// Returns a list of all non-empty section coordinates currently loaded.
    pub fn get_all_non_empty_sections(&self) -> Vec<IVec3> {
        self.sections
            .iter()
            .filter(|(_, sec)| !sec.is_empty())
            .map(|(coord, _)| *coord)
            .collect()
    }

    /// Builds a 18x18x18 padded array for a target section coordinate.
    pub fn get_section_padded_array(&self, coord: IVec3) -> PaddedVoxelArray {
        let default_sec = SectionStorage::new(coord);
        let sec = self.sections.get(&coord).unwrap_or(&default_sec);

        let sec_wx = coord.x * 16;
        let sec_wy = coord.y * 16;
        let sec_wz = coord.z * 16;

        sec.build_padded_array(|lx, ly, lz| {
            let wx = sec_wx + lx;
            let wy = sec_wy + ly;
            let wz = sec_wz + lz;
            self.get_block(wx, wy, wz).to_string()
        })
    }
}
