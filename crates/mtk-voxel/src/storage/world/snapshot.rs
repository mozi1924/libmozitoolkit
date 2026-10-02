use glam::IVec3;

use crate::crc::extract_canonical_state_str;
use crate::storage::SectionStorage;
use super::container::VoxelStorage;

impl VoxelStorage {
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
                            self.biome_column_map.insert([wx, wz], bp[b_idx].clone());
                        }
                    }
                }
            }
        }

        self.mark_all_sections_dirty();
    }

    /// Ingests a single 16x16x16 section snapshot packet and updates dirty tracking.
    pub fn set_section_snapshot(
        &mut self,
        sec_x: i32,
        sec_y: i32,
        sec_z: i32,
        start_x: i32,
        start_y: i32,
        start_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: &[String],
        grid_indices: &[u16],
        biome_palette: Option<&[String]>,
        biome_indices: Option<&[u16]>,
    ) -> bool {
        if size_x <= 0 || size_y <= 0 || size_z <= 0 {
            return false;
        }

        if self.size_x == 0 || self.size_y == 0 || self.size_z == 0 {
            self.min_x = start_x;
            self.min_y = start_y;
            self.min_z = start_z;
            self.size_x = size_x;
            self.size_y = size_y;
            self.size_z = size_z;
        } else {
            let cur_max_x = self.min_x + self.size_x;
            let cur_max_y = self.min_y + self.size_y;
            let cur_max_z = self.min_z + self.size_z;

            let new_min_x = self.min_x.min(start_x);
            let new_min_y = self.min_y.min(start_y);
            let new_min_z = self.min_z.min(start_z);

            let new_max_x = cur_max_x.max(start_x + size_x);
            let new_max_y = cur_max_y.max(start_y + size_y);
            let new_max_z = cur_max_z.max(start_z + size_z);

            self.min_x = new_min_x;
            self.min_y = new_min_y;
            self.min_z = new_min_z;
            self.size_x = new_max_x - new_min_x;
            self.size_y = new_max_y - new_min_y;
            self.size_z = new_max_z - new_min_z;
        }

        let total_blocks = (size_x * size_y * size_z) as usize;
        let p_len = palette.len();
        if grid_indices.len() < total_blocks {
            return false;
        }

        let has_biomes = biome_palette.is_some() && biome_indices.is_some();
        if let Some(bp) = biome_palette {
            if let Some(first) = bp.first() {
                if self.primary_biome.is_none() {
                    self.primary_biome = Some(first.clone());
                }
            }
        }

        let sec_coord = IVec3::new(sec_x, sec_y, sec_z);
        let sec = self
            .sections
            .entry(sec_coord)
            .or_insert_with(|| SectionStorage::new(sec_coord));

        for idx in 0..total_blocks {
            let p_idx = grid_indices[idx] as usize;
            if p_idx >= p_len {
                continue;
            }
            let state = &palette[p_idx];

            let rem = idx % (size_y * size_z) as usize;
            let lx = idx / (size_y * size_z) as usize;
            let ly = rem / size_z as usize;
            let lz = rem % size_z as usize;

            let wx = start_x + lx as i32;
            let wy = start_y + ly as i32;
            let wz = start_z + lz as i32;

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
                            self.biome_column_map.insert([wx, wz], bp[b_idx].clone());
                        }
                    }
                }
            }
        }

        // Compute section CRC and mark dirty with 3x3x3 neighborhood halo
        let crc = sec.compute_crc();
        self.section_crc_map.insert(sec_coord, crc);

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    self.dirty_sections.insert(sec_coord + IVec3::new(dx, dy, dz));
                }
            }
        }

        self.advance_generation();
        true
    }

    /// Checks if incoming full snapshot data matches current memory state without mutations.
    pub fn is_snapshot_identical(
        &self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: &[String],
        grid_indices: &[u16],
    ) -> bool {
        if self.min_x != min_x
            || self.min_y != min_y
            || self.min_z != min_z
            || self.size_x != size_x
            || self.size_y != size_y
            || self.size_z != size_z
        {
            return false;
        }

        let total_blocks = (size_x * size_y * size_z) as usize;
        let p_len = palette.len();
        if grid_indices.len() < total_blocks {
            return false;
        }

        for idx in 0..total_blocks {
            let p_idx = grid_indices[idx] as usize;
            if p_idx >= p_len {
                return false;
            }
            let expected_state = extract_canonical_state_str(&palette[p_idx]);

            let rem = idx % (size_y * size_z) as usize;
            let lx = idx / (size_y * size_z) as usize;
            let ly = rem / size_z as usize;
            let lz = rem % size_z as usize;

            let wx = min_x + lx as i32;
            let wy = min_y + ly as i32;
            let wz = min_z + lz as i32;

            let actual_state = self.get_block(wx, wy, wz);
            if actual_state != expected_state {
                return false;
            }
        }

        true
    }
}
