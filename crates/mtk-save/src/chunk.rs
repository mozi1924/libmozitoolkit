use crate::nbt::{Nbt, NbtCompound};
use glam::IVec3;
use mtk_core::constants::voxel::{block_index, SECTION_VOLUME};
use mtk_voxel::storage::SectionStorage;
use std::io::Cursor;

use crate::error::SaveError;

/// Formats a blockstate and its properties into a canonical sorted blockstate string.
///
/// Example: `minecraft:oak_stairs[facing=north,half=bottom,waterlogged=false]`
pub fn format_canonical_blockstate(block_comp: &NbtCompound) -> String {
    let name = block_comp
        .string("Name")
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "minecraft:air".into());

    let norm_name = match name.as_ref() {
        "cave_air" | "minecraft:cave_air" | "void_air" | "minecraft:void_air" => "minecraft:air",
        other => other,
    };

    if let Some(props) = block_comp.compound("Properties") {
        let mut prop_list: Vec<(String, String)> = Vec::new();
        for (k, v) in props.iter() {
            let key_str = k.to_string_lossy().into_owned();
            let val_str = v
                .string()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            prop_list.push((key_str, val_str));
        }

        if !prop_list.is_empty() {
            prop_list.sort_by(|a, b| a.0.cmp(&b.0));
            let prop_str = prop_list
                .into_iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",");
            return format!("{}[{}]", norm_name, prop_str);
        }
    }

    norm_name.to_string()
}

/// Parses a modern Minecraft 1.18+ chunk and yields populated `SectionStorage` instances.
pub struct ChunkParser;

impl ChunkParser {
    /// Parses a single section from a section compound tag.
    ///
    /// Returns `Ok(None)` if the section is entirely air or unpopulated.
    pub fn parse_section(
        chunk_coord: IVec3,
        section_comp: &NbtCompound,
    ) -> Result<Option<SectionStorage>, SaveError> {
        let sec_y = section_comp.byte("Y").ok_or_else(|| {
            SaveError::InvalidChunkData("Section missing 'Y' coordinate byte".to_string())
        })? as i32;

        let bs = match section_comp.compound("block_states") {
            Some(bs) => bs,
            None => return Ok(None),
        };

        let palette_compounds = match bs.list("palette").and_then(|p| p.compounds()) {
            Some(pal) => pal,
            None => return Ok(None),
        };

        if palette_compounds.is_empty() {
            return Ok(None);
        }

        // 1. Build palette of canonical blockstate strings
        let mut palette: Vec<String> = Vec::with_capacity(palette_compounds.len());
        let mut all_air = true;
        for b in palette_compounds {
            let state = format_canonical_blockstate(&b);
            if state != "minecraft:air" {
                all_air = false;
            }
            palette.push(state);
        }

        // Fast skip if the entire section palette contains only air
        if all_air {
            return Ok(None);
        }

        let section_world_coord = IVec3::new(chunk_coord.x, sec_y, chunk_coord.z);
        let mut section = SectionStorage::new(section_world_coord);

        let data_longs = bs.long_array("data");
        if palette.len() == 1 || data_longs.is_none() {
            // Entire 16x16x16 section is filled with a single state
            let single_state = &palette[0];
            let pal_id = 1u16;
            section.palette.push(single_state.clone());
            section.voxels.fill(pal_id);
            section.non_air_count = SECTION_VOLUME as u32;
            section.cached_crc = None;
            return Ok(Some(section));
        }

        let data = data_longs.unwrap();
        let bits_per_block =
            (32 - ((palette.len() as u32).saturating_sub(1)).leading_zeros()).max(4) as usize;
        let blocks_per_long = 64 / bits_per_block;
        let mask = (1u64 << bits_per_block) - 1;

        // Map section palette to SectionStorage palette (slot 0 is always air)
        let mut pal_map: Vec<u16> = Vec::with_capacity(palette.len());
        for st in &palette {
            if st == "minecraft:air" {
                pal_map.push(0);
            } else {
                let id = section.palette.len() as u16;
                section.palette.push(st.clone());
                pal_map.push(id);
            }
        }

        let mut non_air = 0u32;
        let total_blocks = SECTION_VOLUME;

        for i in 0..total_blocks {
            let long_idx = i / blocks_per_long;
            if long_idx >= data.len() {
                break;
            }
            let bit_offset = (i % blocks_per_long) * bits_per_block;
            let raw_pal_idx = ((data[long_idx] as u64 >> bit_offset) & mask) as usize;

            if raw_pal_idx < pal_map.len() {
                let target_pal_id = pal_map[raw_pal_idx];
                if target_pal_id != 0 {
                    // Minecraft chunk layout: i = (y << 8) | (z << 4) | x
                    let lx = i & 15;
                    let lz = (i >> 4) & 15;
                    let ly = (i >> 8) & 15;

                    // mtk-core internal coordinate index: block_index(x, y, z)
                    let internal_idx = block_index(lx, ly, lz);
                    section.voxels[internal_idx] = target_pal_id;
                    non_air += 1;
                }
            }
        }

        section.non_air_count = non_air;
        section.cached_crc = None;

        if non_air > 0 {
            Ok(Some(section))
        } else {
            Ok(None)
        }
    }

    /// Parses decompressed chunk bytes, querying only sections whose Y coordinate falls
    /// within `min_section_y..=max_section_y`.
    pub fn parse_chunk_sections(
        decompressed_bytes: &[u8],
        min_section_y: i32,
        max_section_y: i32,
    ) -> Result<Vec<SectionStorage>, SaveError> {
        let nbt = crate::nbt::read(&mut Cursor::new(decompressed_bytes))?;
        let base = match nbt {
            Nbt::Some(b) => b,
            Nbt::None => return Ok(Vec::new()),
        };

        let chunk = base.as_compound();
        let chunk_x = chunk.int("xPos").unwrap_or(0);
        let chunk_z = chunk.int("zPos").unwrap_or(0);
        let chunk_coord = IVec3::new(chunk_x, 0, chunk_z);

        let sections_list = match chunk.list("sections").and_then(|l| l.compounds()) {
            Some(list) => list,
            None => return Ok(Vec::new()),
        };

        let mut result = Vec::new();
        for sec_comp in sections_list {
            if let Some(y_byte) = sec_comp.byte("Y") {
                let sec_y = y_byte as i32;
                if sec_y >= min_section_y && sec_y <= max_section_y {
                    if let Some(section) = Self::parse_section(chunk_coord, &sec_comp)? {
                        result.push(section);
                    }
                }
            }
        }

        Ok(result)
    }
}
