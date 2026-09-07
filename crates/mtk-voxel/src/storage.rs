use std::collections::HashMap;

use glam::IVec3;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::crc::{crc32_update, extract_canonical_state_str, EMPTY_SECTION_CRC};

/// Dimension of standard Minecraft sub-chunk section.
pub const SECTION_SIZE: usize = 16;
/// Total number of voxels in a 16x16x16 section.
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE; // 4096
/// Padded dimension with 1-voxel apron on each side for branchless boundary lookups.
pub const PADDED_SIZE: usize = 18;
/// Total number of voxels in a 18x18x18 padded volume.
pub const PADDED_VOLUME: usize = PADDED_SIZE * PADDED_SIZE * PADDED_SIZE; // 5832

/// Canonical local block linear index inside a 16x16x16 section: `x * 256 + y * 16 + z`.
#[inline(always)]
pub const fn block_index(x: usize, y: usize, z: usize) -> usize {
    x * 256 + y * 16 + z
}

/// Canonical padded block linear index inside a 18x18x18 volume: `px * 324 + py * 18 + pz`.
#[inline(always)]
pub const fn padded_index(px: usize, py: usize, pz: usize) -> usize {
    px * 324 + py * 18 + pz
}

/// 16x16x16 Chunk Section with dense palette-indexed storage and non-air tracking.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SectionStorage {
    /// Section coordinate in 16-block chunk grid (e.g. `[cx, cy, cz]`).
    pub coord: IVec3,
    /// Palette mapping index `u16` -> BlockState string (slot 0 is always "minecraft:air").
    pub palette: Vec<String>,
    /// Dense 4096-element palette-indexed array (`x * 256 + y * 16 + z`).
    pub voxels: Box<[u16; SECTION_VOLUME]>,
    /// Number of non-air voxels currently in this section.
    pub non_air_count: u32,
    /// Cached CRC32 checksum for this section.
    pub cached_crc: Option<u32>,
}

impl SectionStorage {
    /// Creates an empty section filled with `minecraft:air` at coordinate `coord`.
    pub fn new(coord: IVec3) -> Self {
        Self {
            coord,
            palette: vec!["minecraft:air".to_string()],
            voxels: Box::new([0u16; SECTION_VOLUME]),
            non_air_count: 0,
            cached_crc: Some(EMPTY_SECTION_CRC),
        }
    }

    /// Creates a section pre-populated from a 4096-element blockstate slice.
    pub fn from_slice(coord: IVec3, states: &[&str]) -> Self {
        let mut section = Self::new(coord);
        let mut palette_map = HashMap::<String, u16>::new();
        palette_map.insert("minecraft:air".to_string(), 0);

        let count = states.len().min(SECTION_VOLUME);
        for idx in 0..count {
            let state_canon = extract_canonical_state_str(states[idx]);
            let pal_idx = if let Some(&id) = palette_map.get(state_canon) {
                id
            } else {
                let id = section.palette.len() as u16;
                section.palette.push(state_canon.to_string());
                palette_map.insert(state_canon.to_string(), id);
                id
            };

            section.voxels[idx] = pal_idx;
            if pal_idx != 0 {
                section.non_air_count += 1;
            }
        }
        section.cached_crc = None;
        section
    }

    /// Checks if this section is known to be entirely empty (all air).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.non_air_count == 0
    }

    /// Returns the palette index at local section coordinate `(x, y, z)` in `0..16`.
    #[inline]
    pub fn get_local_idx(&self, x: usize, y: usize, z: usize) -> u16 {
        if x < SECTION_SIZE && y < SECTION_SIZE && z < SECTION_SIZE {
            self.voxels[block_index(x, y, z)]
        } else {
            0
        }
    }

    /// Returns the blockstate string at local section coordinate `(x, y, z)` in `0..16`.
    #[inline]
    pub fn get_local_state(&self, x: usize, y: usize, z: usize) -> &str {
        let pal_idx = self.get_local_idx(x, y, z) as usize;
        if pal_idx < self.palette.len() {
            &self.palette[pal_idx]
        } else {
            "minecraft:air"
        }
    }

    /// Sets the blockstate at local section coordinate `(x, y, z)` in `0..16`.
    /// Returns `true` if the voxel changed state.
    pub fn set_local(&mut self, x: usize, y: usize, z: usize, state: &str) -> bool {
        if x >= SECTION_SIZE || y >= SECTION_SIZE || z >= SECTION_SIZE {
            return false;
        }

        let state_canon = extract_canonical_state_str(state);
        let idx = block_index(x, y, z);
        let old_pal_idx = self.voxels[idx];
        let old_state = &self.palette[old_pal_idx as usize];

        if old_state == state_canon {
            return false;
        }

        // Find or insert in palette
        let new_pal_idx = if let Some(pos) = self.palette.iter().position(|s| s == state_canon) {
            pos as u16
        } else {
            let id = self.palette.len() as u16;
            self.palette.push(state_canon.to_string());
            id
        };

        if old_pal_idx == 0 && new_pal_idx != 0 {
            self.non_air_count += 1;
        } else if old_pal_idx != 0 && new_pal_idx == 0 {
            self.non_air_count = self.non_air_count.saturating_sub(1);
        }

        self.voxels[idx] = new_pal_idx;
        self.cached_crc = None;
        true
    }

    /// Computes or retrieves the canonical CRC32 checksum for this section.
    pub fn compute_crc(&mut self) -> u32 {
        if let Some(crc) = self.cached_crc {
            return crc;
        }

        if self.is_empty() {
            self.cached_crc = Some(EMPTY_SECTION_CRC);
            return EMPTY_SECTION_CRC;
        }

        let mut crc_val = 0u32;
        for x in 0..SECTION_SIZE {
            for y in 0..SECTION_SIZE {
                for z in 0..SECTION_SIZE {
                    let st = self.get_local_state(x, y, z);
                    crc_val = crc32_update(crc_val, st.as_bytes());
                }
            }
        }

        self.cached_crc = Some(crc_val);
        crc_val
    }

    /// Builds a 18x18x18 padded array with a 1-voxel neighborhood apron.
    ///
    /// `get_neighbor` receives local coordinates `(lx, ly, lz)` in `-1..=16`
    /// for voxels outside the `0..16` section boundary.
    pub fn build_padded_array<F>(&self, mut get_neighbor: F) -> PaddedVoxelArray
    where
        F: FnMut(i32, i32, i32) -> String,
    {
        let mut palette_map = HashMap::<String, u16>::new();
        let mut palette = self.palette.clone();
        for (i, st) in palette.iter().enumerate() {
            palette_map.insert(st.clone(), i as u16);
        }

        let mut get_or_insert = |st: &str| -> u16 {
            if let Some(&id) = palette_map.get(st) {
                id
            } else {
                let id = palette.len() as u16;
                palette.push(st.to_string());
                palette_map.insert(st.to_string(), id);
                id
            }
        };

        let mut padded = vec![0u16; PADDED_VOLUME];

        for px in 0..PADDED_SIZE {
            let lx = px as i32 - 1;
            for py in 0..PADDED_SIZE {
                let ly = py as i32 - 1;
                for pz in 0..PADDED_SIZE {
                    let lz = pz as i32 - 1;
                    let pad_idx = padded_index(px, py, pz);

                    let pal_idx = if lx >= 0 && lx < 16 && ly >= 0 && ly < 16 && lz >= 0 && lz < 16 {
                        let core_idx = block_index(lx as usize, ly as usize, lz as usize);
                        self.voxels[core_idx]
                    } else {
                        let n_state = get_neighbor(lx, ly, lz);
                        get_or_insert(extract_canonical_state_str(&n_state))
                    };

                    padded[pad_idx] = pal_idx;
                }
            }
        }

        PaddedVoxelArray {
            coord: self.coord,
            palette,
            padded_voxels: padded,
            is_empty: self.is_empty(),
        }
    }
}

/// 18x18x18 Padded Voxel Array for zero-branching face culling and neighbor inspection.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaddedVoxelArray {
    /// Section coordinate in chunk grid.
    pub coord: IVec3,
    /// Unified palette for this section and its padded apron.
    pub palette: Vec<String>,
    /// 18x18x18 padded array indexed by `px * 324 + py * 18 + pz`.
    pub padded_voxels: Vec<u16>,
    /// Whether the core section contains only air.
    pub is_empty: bool,
}

impl PaddedVoxelArray {
    /// Returns the blockstate string at padded coordinate `px, py, pz` in `0..18`.
    #[inline(always)]
    pub fn get_padded_state(&self, px: usize, py: usize, pz: usize) -> &str {
        let pad_idx = padded_index(px, py, pz);
        let pal_idx = self.padded_voxels[pad_idx] as usize;
        if pal_idx < self.palette.len() {
            &self.palette[pal_idx]
        } else {
            "minecraft:air"
        }
    }

    /// Returns the palette index at padded coordinate `px, py, pz` in `0..18`.
    #[inline(always)]
    pub fn get_padded_idx(&self, px: usize, py: usize, pz: usize) -> u16 {
        self.padded_voxels[padded_index(px, py, pz)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_storage_basic() {
        let mut sec = SectionStorage::new(IVec3::new(0, 0, 0));
        assert!(sec.is_empty());
        assert_eq!(sec.compute_crc(), EMPTY_SECTION_CRC);

        sec.set_local(0, 0, 0, "minecraft:stone");
        assert!(!sec.is_empty());
        assert_eq!(sec.get_local_state(0, 0, 0), "minecraft:stone");
        assert_ne!(sec.compute_crc(), EMPTY_SECTION_CRC);

        sec.set_local(0, 0, 0, "minecraft:air");
        assert!(sec.is_empty());
        assert_eq!(sec.compute_crc(), EMPTY_SECTION_CRC);
    }

    #[test]
    fn test_padded_array_construction() {
        let mut sec = SectionStorage::new(IVec3::new(1, 2, 3));
        sec.set_local(0, 0, 0, "minecraft:diamond_block");

        let padded = sec.build_padded_array(|lx, ly, lz| {
            if lx == -1 && ly == 0 && lz == 0 {
                "minecraft:gold_block".to_string()
            } else {
                "minecraft:air".to_string()
            }
        });

        assert_eq!(padded.get_padded_state(1, 1, 1), "minecraft:diamond_block");
        assert_eq!(padded.get_padded_state(0, 1, 1), "minecraft:gold_block");
        assert_eq!(padded.get_padded_state(2, 2, 2), "minecraft:air");
    }
}
