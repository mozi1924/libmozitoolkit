//! Dynamic Atlas Chunk for runtime-registered entities (such as player skulls, banners, and paintings).
//!
//! Provides fixed-grid slot allocation (e.g. 1024x1024 atlas accommodating 256 unique 64x64 skins),
//! fast sub-image blitting, automatic UV bounds computation, and integration with `AtlasAddressMap`.

use std::collections::HashMap;

use crate::atlas::address_map::{AtlasAddressMap, AtlasChunkMeta, AtlasSpriteLocation, SpriteKind};
use crate::error::TextureError;

/// Standard dimensions of a dynamic skin atlas chunk.
pub const DYNAMIC_ATLAS_WIDTH: u32 = 1024;
pub const DYNAMIC_ATLAS_HEIGHT: u32 = 1024;
pub const DYNAMIC_SLOT_SIZE: u32 = 64;
pub const DYNAMIC_SLOTS_PER_ROW: u32 = 16;
pub const DYNAMIC_TOTAL_SLOTS: usize = 256;

/// Default reserved chunk ID for the dynamic skin atlas (distinct from static block chunk IDs 0..N).
pub const DEFAULT_DYNAMIC_SKINS_CHUNK_ID: u16 = 0xFFF0;

/// Information about an allocated slot inside a dynamic atlas chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicSlotInfo {
    pub slot_index: u16,
    pub col: u32,
    pub row: u32,
    pub pixel_rect: [u32; 4],
    pub uv_bounds: [f32; 4],
}

/// A fixed-size runtime dynamic atlas chunk that allocates 64x64 grid slots on demand.
#[derive(Debug, Clone)]
pub struct DynamicAtlasChunk {
    pub chunk_id: u16,
    pub width: u32,
    pub height: u32,
    pub rgba_buffer: Vec<u8>,
    slot_map: HashMap<String, u16>,
    next_slot: u16,
}

impl Default for DynamicAtlasChunk {
    fn default() -> Self {
        Self::new(DEFAULT_DYNAMIC_SKINS_CHUNK_ID)
    }
}

impl DynamicAtlasChunk {
    /// Creates a new empty dynamic atlas chunk with the given `chunk_id`.
    pub fn new(chunk_id: u16) -> Self {
        let size = (DYNAMIC_ATLAS_WIDTH * DYNAMIC_ATLAS_HEIGHT * 4) as usize;
        Self {
            chunk_id,
            width: DYNAMIC_ATLAS_WIDTH,
            height: DYNAMIC_ATLAS_HEIGHT,
            rgba_buffer: vec![0u8; size],
            slot_map: HashMap::new(),
            next_slot: 0,
        }
    }

    /// Returns the number of currently allocated unique skin slots.
    pub fn allocated_count(&self) -> usize {
        self.next_slot as usize
    }

    /// Checks if a skin key has already been allocated a slot.
    pub fn contains_key(&self, skin_key: &str) -> bool {
        self.slot_map.contains_key(skin_key)
    }

    /// Gets the slot index for a skin key, if already allocated.
    pub fn get_slot(&self, skin_key: &str) -> Option<u16> {
        self.slot_map.get(skin_key).copied()
    }

    /// Allocates or retrieves an existing slot for a skin key (natural deduplication).
    ///
    /// Returns the slot index (0..255).
    pub fn allocate_slot(&mut self, skin_key: &str) -> u16 {
        if let Some(&slot_idx) = self.slot_map.get(skin_key) {
            return slot_idx;
        }

        let slot_idx = if (self.next_slot as usize) < DYNAMIC_TOTAL_SLOTS {
            let s = self.next_slot;
            self.next_slot += 1;
            s
        } else {
            // Reached maximum capacity (256 slots); wrap around in round-robin sequence
            let s = (self.next_slot as usize % DYNAMIC_TOTAL_SLOTS) as u16;
            self.next_slot = self.next_slot.wrapping_add(1);
            // Purge stale reverse entries previously mapping to this slot
            self.slot_map.retain(|_, &mut v| v != s);
            s
        };

        self.slot_map.insert(skin_key.to_string(), slot_idx);
        slot_idx
    }

    /// Calculates physical pixel coordinates and normalized UV bounds for a given slot.
    pub fn slot_info(slot_idx: u16) -> DynamicSlotInfo {
        let col = (slot_idx as u32) % DYNAMIC_SLOTS_PER_ROW;
        let row = (slot_idx as u32) / DYNAMIC_SLOTS_PER_ROW;
        let px = col * DYNAMIC_SLOT_SIZE;
        let py = row * DYNAMIC_SLOT_SIZE;

        let u_min = px as f32 / DYNAMIC_ATLAS_WIDTH as f32;
        let u_max = (px + DYNAMIC_SLOT_SIZE) as f32 / DYNAMIC_ATLAS_WIDTH as f32;
        let v_min = 1.0 - (py + DYNAMIC_SLOT_SIZE) as f32 / DYNAMIC_ATLAS_HEIGHT as f32;
        let v_max = 1.0 - py as f32 / DYNAMIC_ATLAS_HEIGHT as f32;

        DynamicSlotInfo {
            slot_index: slot_idx,
            col,
            row,
            pixel_rect: [px, py, DYNAMIC_SLOT_SIZE, DYNAMIC_SLOT_SIZE],
            uv_bounds: [u_min, v_min, u_max, v_max],
        }
    }

    /// Blits a 64x64 (or 64x32) raw RGBA pixel buffer directly into the allocated slot.
    pub fn blit_skin_rgba(
        &mut self,
        slot_idx: u16,
        skin_rgba: &[u8],
        skin_width: u32,
        skin_height: u32,
    ) -> Result<DynamicSlotInfo, TextureError> {
        let info = Self::slot_info(slot_idx);

        if skin_width != 64 || (skin_height != 64 && skin_height != 32) {
            return Err(TextureError::Baking(format!(
                "Invalid skin dimensions: {}x{} (must be 64x64 or 64x32)",
                skin_width, skin_height
            )));
        }

        let expected_len = (skin_width * skin_height * 4) as usize;
        if skin_rgba.len() < expected_len {
            return Err(TextureError::Baking(format!(
                "Skin RGBA buffer too small: expected {}, got {}",
                expected_len,
                skin_rgba.len()
            )));
        }

        let base_x = info.pixel_rect[0] as usize;
        let base_y = info.pixel_rect[1] as usize;
        let atlas_stride = (DYNAMIC_ATLAS_WIDTH * 4) as usize;
        let row_bytes = (skin_width * 4) as usize;

        for y in 0..(skin_height as usize) {
            let dst_offset = (base_y + y) * atlas_stride + base_x * 4;
            let src_offset = y * row_bytes;
            self.rgba_buffer[dst_offset..dst_offset + row_bytes]
                .copy_from_slice(&skin_rgba[src_offset..src_offset + row_bytes]);
        }

        // If legacy 64x32 skin, clear or mirror lower 32 rows if needed (leaving transparent)
        if skin_height == 32 {
            for y in 32..64 {
                let dst_offset = (base_y + y) * atlas_stride + base_x * 4;
                self.rgba_buffer[dst_offset..dst_offset + row_bytes].fill(0);
            }
        }

        Ok(info)
    }

    /// Decodes a PNG image from memory and blits it into the specified slot.
    pub fn blit_skin_png_bytes(
        &mut self,
        slot_idx: u16,
        png_bytes: &[u8],
    ) -> Result<DynamicSlotInfo, TextureError> {
        let dyn_img = image::load_from_memory(png_bytes).map_err(TextureError::Image)?;
        let rgba = dyn_img.to_rgba8();
        self.blit_skin_rgba(slot_idx, rgba.as_raw(), rgba.width(), rgba.height())
    }

    /// Associates an alias key with an already allocated slot or another key.
    pub fn add_alias(&mut self, alias_key: &str, target_key: &str) {
        if let Some(&slot_idx) = self.slot_map.get(target_key) {
            self.slot_map.insert(alias_key.to_string(), slot_idx);
        }
    }

    /// Convenience helper to allocate for primary hash, map aliases to that slot, and blit the skin PNG.
    pub fn add_skin_png_with_aliases(
        &mut self,
        primary_key: &str,
        aliases: &[&str],
        png_bytes: &[u8],
    ) -> Result<DynamicSlotInfo, TextureError> {
        let slot_idx = self.allocate_slot(primary_key);
        for &alias in aliases {
            if !alias.is_empty() {
                self.slot_map.insert(alias.to_string(), slot_idx);
            }
        }
        self.blit_skin_png_bytes(slot_idx, png_bytes)
    }

    /// Convenience helper to allocate and blit a skin in one step.
    pub fn add_skin_png(
        &mut self,
        skin_key: &str,
        png_bytes: &[u8],
    ) -> Result<DynamicSlotInfo, TextureError> {
        let slot_idx = self.allocate_slot(skin_key);
        self.blit_skin_png_bytes(slot_idx, png_bytes)
    }

    /// Registers all currently allocated slots into the given `AtlasAddressMap`.
    ///
    /// Maps both `minecraft:skin/<key>` and direct `<key>` aliases so that
    /// both ModelBaker textures and Direct texture identifiers resolve instantly.
    pub fn register_to_address_map(&self, address_map: &mut AtlasAddressMap) {
        // Ensure the dynamic chunk meta is registered
        if !address_map
            .chunks
            .iter()
            .any(|c| c.chunk_id == self.chunk_id)
        {
            address_map.chunks.push(self.to_chunk_meta());
        }

        for (skin_key, &slot_idx) in &self.slot_map {
            let info = Self::slot_info(slot_idx);
            let loc = AtlasSpriteLocation {
                chunk_id: self.chunk_id,
                category: "skins".to_string(),
                is_animated: false,
                sprite_kind: SpriteKind::StaticAtlas,
                texture_id: 0,
                uv_bounds: info.uv_bounds,
                frame_0_uv_bounds: info.uv_bounds,
                local_uv_bounds: [0.0, 0.0, 1.0, 1.0],
                frame_uv_step: [0.0, 0.0],
                pixel_rect: info.pixel_rect,
                strip_pixel_rect: info.pixel_rect,
                frame_size: [DYNAMIC_SLOT_SIZE, DYNAMIC_SLOT_SIZE],
                frame_count: 1,
                animation: None,
                has_normal: false,
                has_specular: false,
                has_overlay: false,
                average_color: None,
            };

            let variants = [
                format!("minecraft:skin/{}", skin_key),
                format!("skin/{}", skin_key),
                format!("minecraft:player/{}", skin_key),
                format!("player/{}", skin_key),
                format!("entity/player/{}", skin_key),
                skin_key.clone(),
            ];

            for v in variants {
                if let Ok(loc_key) = mtk_resource::ResourceLocation::parse(&v) {
                    address_map.sprites.insert(loc_key, loc.clone());
                }
            }
        }
    }

    /// Produces AtlasChunkMeta representation for Blender DCC material binding.
    pub fn to_chunk_meta(&self) -> AtlasChunkMeta {
        AtlasChunkMeta {
            chunk_id: self.chunk_id,
            category: "skins".to_string(),
            is_animated: false,
            category_chunk_index: 1,
            width: self.width,
            height: self.height,
            has_normal: false,
            has_specular: false,
            has_overlay: false,
            has_orm: false,
            has_aux: false,
            has_tint: false,
            pbr: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_atlas_allocation_and_deduplication() {
        let mut atlas = DynamicAtlasChunk::new(0xFFF0);
        let s1 = atlas.allocate_slot("alex");
        let s2 = atlas.allocate_slot("steve");
        let s1_again = atlas.allocate_slot("alex");

        assert_eq!(s1, 0);
        assert_eq!(s2, 1);
        assert_eq!(s1_again, 0); // Natural deduplication
        assert_eq!(atlas.allocated_count(), 2);
    }

    #[test]
    fn test_dynamic_atlas_slot_uv_bounds() {
        let info0 = DynamicAtlasChunk::slot_info(0);
        assert_eq!(info0.col, 0);
        assert_eq!(info0.row, 0);
        assert_eq!(info0.pixel_rect, [0, 0, 64, 64]);
        assert_eq!(info0.uv_bounds, [0.0, 0.9375, 0.0625, 1.0]);

        let info17 = DynamicAtlasChunk::slot_info(17); // row 1, col 1
        assert_eq!(info17.col, 1);
        assert_eq!(info17.row, 1);
        assert_eq!(info17.pixel_rect, [64, 64, 64, 64]);
        assert_eq!(info17.uv_bounds, [0.0625, 0.875, 0.125, 0.9375]);
    }

    #[test]
    fn test_dynamic_atlas_blit_and_address_map() {
        let mut atlas = DynamicAtlasChunk::new(0xFFF0);
        let dummy_skin_rgba = vec![255u8; 64 * 64 * 4];
        let info = atlas
            .blit_skin_rgba(0, &dummy_skin_rgba, 64, 64)
            .expect("Blit should succeed");

        assert_eq!(info.slot_index, 0);
        atlas.slot_map.insert("player_test".to_string(), 0);

        let mut addr_map = AtlasAddressMap::new();
        atlas.register_to_address_map(&mut addr_map);

        let loc = addr_map
            .lookup_str("minecraft:skin/player_test")
            .expect("Sprite must be registered");
        assert_eq!(loc.chunk_id, 0xFFF0);
        assert_eq!(loc.uv_bounds, [0.0, 0.9375, 0.0625, 1.0]);
    }

    #[test]
    fn test_dynamic_atlas_overflow_round_robin() {
        let mut atlas = DynamicAtlasChunk::new(0xFFF0);
        for i in 0..256 {
            let slot = atlas.allocate_slot(&format!("skin_{}", i));
            assert_eq!(slot, i as u16);
        }
        // Slot 256 wraps around to 0
        let slot_256 = atlas.allocate_slot("skin_256");
        assert_eq!(slot_256, 0);
        // Slot 257 wraps around to 1
        let slot_257 = atlas.allocate_slot("skin_257");
        assert_eq!(slot_257, 1);
        // Stale skin_0 should have been purged, skin_256 now owns slot 0
        assert_eq!(atlas.get_slot("skin_256"), Some(0));
        assert_eq!(atlas.get_slot("skin_0"), None);
    }
}
