use std::collections::HashSet;

use glam::IVec3;

use super::container::VoxelStorage;

impl VoxelStorage {
    /// Computes and caches CRC32 for a single section, clamped to active selection bounds.
    pub fn calculate_and_store_section_crc(&mut self, coord: IVec3) -> u32 {
        let (start_x, start_y, start_z, sx, sy, sz) = self.get_section_block_bounds(coord);
        let total_blocks = (sx * sy * sz) as usize;
        if total_blocks == 0 {
            self.section_crc_map.insert(coord, 0);
            return 0;
        }

        let is_full_section = sx == 16 && sy == 16 && sz == 16;
        let sec = self.sections.get_mut(&coord);

        let crc = if let Some(s) = sec {
            if s.is_empty() {
                crate::crc::get_empty_section_crc(total_blocks)
            } else if is_full_section {
                s.compute_crc()
            } else {
                let end_x = start_x + sx - 1;
                let end_y = start_y + sy - 1;
                let end_z = start_z + sz - 1;
                let mut crc_val = 0u32;
                for x in start_x..=end_x {
                    let lx = (x & 15) as usize;
                    for y in start_y..=end_y {
                        let ly = (y & 15) as usize;
                        for z in start_z..=end_z {
                            let lz = (z & 15) as usize;
                            let st = s.get_local_state(lx, ly, lz);
                            crc_val = crate::crc::crc32_update(crc_val, st.as_bytes());
                        }
                    }
                }
                crc_val
            }
        } else {
            crate::crc::get_empty_section_crc(total_blocks)
        };

        self.section_crc_map.insert(coord, crc);
        crc
    }

    /// Recomputes CRC32 for all sections currently loaded.
    pub fn recalculate_all_section_crcs(&mut self) {
        self.section_crc_map.clear();
        let coords: Vec<IVec3> = self.sections.keys().copied().collect();
        for coord in coords {
            self.calculate_and_store_section_crc(coord);
        }
    }

    /// Returns bounding box coordinates and sizes for a given section clamped to world selection.
    pub fn get_section_block_bounds(&self, coord: IVec3) -> (i32, i32, i32, i32, i32, i32) {
        if self.size_x <= 0 || self.size_y <= 0 || self.size_z <= 0 {
            return (coord.x << 4, coord.y << 4, coord.z << 4, 16, 16, 16);
        }
        let max_x = self.min_x + self.size_x - 1;
        let max_y = self.min_y + self.size_y - 1;
        let max_z = self.min_z + self.size_z - 1;

        let start_x = self.min_x.max(coord.x << 4);
        let end_x = max_x.min((coord.x << 4) + 15);
        let start_y = self.min_y.max(coord.y << 4);
        let end_y = max_y.min((coord.y << 4) + 15);
        let start_z = self.min_z.max(coord.z << 4);
        let end_z = max_z.min((coord.z << 4) + 15);

        let sx = (end_x - start_x + 1).max(0);
        let sy = (end_y - start_y + 1).max(0);
        let sz = (end_z - start_z + 1).max(0);
        (start_x, start_y, start_z, sx, sy, sz)
    }

    /// Returns total block volume for a given section clamped to active selection bounds.
    pub fn get_section_block_count(&self, coord: IVec3) -> usize {
        let (_, _, _, sx, sy, sz) = self.get_section_block_bounds(coord);
        (sx * sy * sz) as usize
    }

    /// Checks if a CRC32 matches the canonical empty air CRC for this section's clamped volume.
    pub fn is_empty_section_crc(&self, coord: IVec3, crc_val: u32) -> bool {
        let count = self.get_section_block_count(coord);
        crc_val == crate::crc::get_empty_section_crc(count)
    }

    /// Compares server section CRC32 hashes with local ones and identifies out-of-sync sections.
    pub fn validate_manifest(
        &mut self,
        server_sections: &[(i32, i32, i32, u32)],
        existing_section_meshes: Option<&HashSet<IVec3>>,
    ) -> Vec<IVec3> {
        let mut mismatched = Vec::new();

        for &(sx, sy, sz, server_crc) in server_sections {
            let coord = IVec3::new(sx, sy, sz);
            if self.dirty_sections.contains(&coord) || !self.section_crc_map.contains_key(&coord) {
                self.calculate_and_store_section_crc(coord);
            }

            let local_crc = self.section_crc_map.get(&coord).copied().unwrap_or(0);
            if local_crc != server_crc {
                mismatched.push(coord);
                continue;
            }

            // Bad chunk check: if non-empty section exists on server but its mesh object is missing
            if let Some(existing_meshes) = existing_section_meshes {
                if !self.is_empty_section_crc(coord, server_crc)
                    && !existing_meshes.contains(&coord)
                    && !self.known_empty_sections.contains(&coord)
                {
                    mismatched.push(coord);
                }
            }
        }

        mismatched
    }

    /// Exports manifest metadata (bounds and CRC map) as JSON for scene persistence.
    #[cfg(feature = "serde")]
    pub fn export_manifest_metadata(&self) -> serde_json::Value {
        let mut crc_map = serde_json::Map::new();
        for (&coord, &crc) in &self.section_crc_map {
            let key = format!("{},{},{}", coord.x, coord.y, coord.z);
            crc_map.insert(key, serde_json::Value::from(crc));
        }

        let empty_list: Vec<String> = self
            .known_empty_sections
            .iter()
            .map(|c| format!("{},{},{}", c.x, c.y, c.z))
            .collect();

        serde_json::json!({
            "min_x": self.min_x,
            "min_y": self.min_y,
            "min_z": self.min_z,
            "size_x": self.size_x,
            "size_y": self.size_y,
            "size_z": self.size_z,
            "generation": self.get_generation(),
            "section_crcs": crc_map,
            "known_empty_sections": empty_list,
        })
    }

    /// Imports manifest metadata from scene JSON properties.
    #[cfg(feature = "serde")]
    pub fn import_manifest_metadata(&mut self, data: &serde_json::Value) -> bool {
        let obj = match data.as_object() {
            Some(o) => o,
            None => return false,
        };

        self.min_x = obj.get("min_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        self.min_y = obj.get("min_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        self.min_z = obj.get("min_z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        self.size_x = obj.get("size_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        self.size_y = obj.get("size_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        self.size_z = obj.get("size_z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let gen = obj.get("generation").and_then(|v| v.as_u64()).unwrap_or(0);
        self.set_generation(gen);

        self.section_crc_map.clear();
        if let Some(crcs) = obj.get("section_crcs").and_then(|v| v.as_object()) {
            for (k, v) in crcs {
                let parts: Vec<&str> = k.split(',').collect();
                if parts.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z), Some(crc_val)) = (
                        parts[0].parse::<i32>(),
                        parts[1].parse::<i32>(),
                        parts[2].parse::<i32>(),
                        v.as_u64(),
                    ) {
                        self.section_crc_map
                            .insert(IVec3::new(x, y, z), crc_val as u32);
                    }
                }
            }
        }

        self.known_empty_sections.clear();
        if let Some(empty_arr) = obj.get("known_empty_sections").and_then(|v| v.as_array()) {
            for v in empty_arr {
                if let Some(s) = v.as_str() {
                    let parts: Vec<&str> = s.split(',').collect();
                    if parts.len() == 3 {
                        if let (Ok(x), Ok(y), Ok(z)) = (
                            parts[0].parse::<i32>(),
                            parts[1].parse::<i32>(),
                            parts[2].parse::<i32>(),
                        ) {
                            self.known_empty_sections.insert(IVec3::new(x, y, z));
                        }
                    }
                }
            }
        }

        true
    }

    /// Ingests sections from an external `VoxelSource` provider.
    pub fn ingest_source<S: crate::source::VoxelSource + ?Sized>(
        &mut self,
        source: &mut S,
        sections: Option<&[IVec3]>,
    ) -> Result<usize, crate::types::VoxelError> {
        crate::source::ingest_from_source(source, self, sections)
    }
}
