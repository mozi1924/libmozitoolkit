use glam::IVec3;

use super::container::VoxelStorage;
use crate::storage::{PaddedVoxelArray, SectionStorage};

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
            self.get_block(wx, wy, wz)
        });

        if self.has_explicit_bounds {
            for lx in 0..16 {
                let wx = sec_wx + lx as i32;
                for ly in 0..16 {
                    let wy = sec_wy + ly as i32;
                    for lz in 0..16 {
                        let wz = sec_wz + lz as i32;
                        if !self.contains(wx, wy, wz) {
                            let p_idx = (lx + 1) * 324 + (ly + 1) * 18 + (lz + 1);
                            padded.padded_voxels[p_idx] = 0;
                        }
                    }
                }
            }
        }

        let has_biomes = sec.biomes.is_some()
            || !self.biome_map.is_empty()
            || self.primary_biome.is_some()
            || !self.biome_column_map.is_empty();

        if !padded.is_empty && has_biomes {
            let (min_x, _, min_z, size_x, _, size_z) = self.get_bounds();
            let has_bounds = size_x > 0 && size_z > 0;
            let min_bound_x = min_x;
            let max_bound_x = min_x + size_x - 1;
            let min_bound_z = min_z;
            let max_bound_z = min_z + size_z - 1;

            let is_3d = sec.biomes.as_ref().is_some_and(|b| b.varies_vertically());

            if is_3d {
                let mut biome_cols = Vec::with_capacity(1024);
                for qy in 0..4 {
                    let sample_y = sec_wy + (qy * 4) + 2;
                    for lx in 0..16 {
                        let wx = sec_wx + lx;
                        for lz in 0..16 {
                            let wz = sec_wz + lz;
                            let center_b = self.get_biome(wx, sample_y, wz);
                            let col = crate::biome::get_smoothed_column_biome(
                                |bx, y, bz| {
                                    let cbx = if has_bounds {
                                        bx.clamp(min_bound_x, max_bound_x)
                                    } else {
                                        bx
                                    };
                                    let cbz = if has_bounds {
                                        bz.clamp(min_bound_z, max_bound_z)
                                    } else {
                                        bz
                                    };
                                    let b = self.get_biome(cbx, y, cbz);
                                    if b == "minecraft:plains"
                                        && center_b != "minecraft:plains"
                                        && !self.biome_column_map.contains_key(&[cbx, cbz])
                                        && !self.biome_map.contains_key(&IVec3::new(cbx, y, cbz))
                                        && !self.sections.contains_key(&IVec3::new(
                                            cbx >> 4,
                                            y >> 4,
                                            cbz >> 4,
                                        ))
                                    {
                                        return center_b;
                                    }
                                    b
                                },
                                wx,
                                sample_y,
                                wz,
                            );
                            biome_cols.push(col);
                        }
                    }
                }
                padded.biome_data = Some(biome_cols);
            } else {
                let mut biome_cols = Vec::with_capacity(256);
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
                                sample_y = sec_wy + ly;
                                break;
                            }
                        }

                        let center_b = self.get_biome(wx, sample_y, wz);

                        let col = crate::biome::get_smoothed_column_biome(
                            |bx, y, bz| {
                                let cbx = if has_bounds {
                                    bx.clamp(min_bound_x, max_bound_x)
                                } else {
                                    bx
                                };
                                let cbz = if has_bounds {
                                    bz.clamp(min_bound_z, max_bound_z)
                                } else {
                                    bz
                                };
                                let b = self.get_biome(cbx, y, cbz);
                                if b == "minecraft:plains"
                                    && center_b != "minecraft:plains"
                                    && !self.biome_column_map.contains_key(&[cbx, cbz])
                                    && !self.biome_map.contains_key(&IVec3::new(cbx, y, cbz))
                                    && !self.sections.contains_key(&IVec3::new(
                                        cbx >> 4,
                                        y >> 4,
                                        cbz >> 4,
                                    ))
                                {
                                    return center_b;
                                }
                                b
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
        }

        padded
    }
}
