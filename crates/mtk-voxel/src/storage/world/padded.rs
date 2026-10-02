use glam::IVec3;

use crate::storage::{PaddedVoxelArray, SectionStorage};
use super::container::VoxelStorage;

impl VoxelStorage {
    /// Builds a 18x18x18 padded array for a target section coordinate.
    pub fn get_section_padded_array(&self, coord: IVec3) -> PaddedVoxelArray {
        let default_sec = SectionStorage::new(coord);
        let sec = self.sections.get(&coord).unwrap_or(&default_sec);

        let sec_wx = coord.x * 16;
        let sec_wy = coord.y * 16;
        let sec_wz = coord.z * 16;

        let mut padded = sec.build_padded_array(|lx, ly, lz| {
            let wx = sec_wx + lx;
            let wy = sec_wy + ly;
            let wz = sec_wz + lz;
            self.get_block(wx, wy, wz).to_string()
        });

        if !padded.is_empty && (!self.biome_map.is_empty() || self.primary_biome.is_some()) {
            let mut biome_cols = Vec::with_capacity(256);
            let (min_x, _, min_z, size_x, _, size_z) = self.get_bounds();
            let has_bounds = size_x > 0 && size_z > 0;
            let min_bound_x = min_x;
            let max_bound_x = min_x + size_x - 1;
            let min_bound_z = min_z;
            let max_bound_z = min_z + size_z - 1;

            let mid_y = sec_wy + 8;
            for lx in 0..16 {
                let wx = sec_wx + lx;
                for lz in 0..16 {
                    let wz = sec_wz + lz;

                    // Prefer block height in this column if non-air block exists, else default to mid_y
                    let mut sample_y = mid_y;
                    for ly in (0..16).rev() {
                        let state = sec.get_local_state(lx as usize, ly as usize, lz as usize);
                        if state != "minecraft:air" && !state.starts_with("minecraft:air") {
                            sample_y = sec_wy + ly as i32;
                            break;
                        }
                    }

                    let center_b = self.get_biome(wx, sample_y, wz);

                    let col = crate::biome::get_smoothed_column_biome(
                        |bx, y, bz| {
                            let cbx = if has_bounds { bx.clamp(min_bound_x, max_bound_x) } else { bx };
                            let cbz = if has_bounds { bz.clamp(min_bound_z, max_bound_z) } else { bz };
                            let b = self.get_biome(cbx, y, cbz);
                            // Avoid erroneous fallback to plains if the sample column had no data but center_b is non-plains
                            if b == "minecraft:plains" && center_b != "minecraft:plains" {
                                if !self.biome_column_map.contains_key(&[cbx, cbz]) && !self.biome_map.contains_key(&IVec3::new(cbx, y, cbz)) {
                                    return center_b.to_string();
                                }
                            }
                            b.to_string()
                        },
                        wx,
                        sample_y,
                        wz,
                    );
                    biome_cols.push(col);
                }
            }
            padded.biome_data = Some(biome_cols);
        }

        padded
    }
}
