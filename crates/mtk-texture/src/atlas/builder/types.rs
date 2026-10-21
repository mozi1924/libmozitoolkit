use crate::atlas::address_map::AtlasAddressMap;
use crate::image::buffer::RgbaBuffer;

/// Configuration parameters for atlas generation.
#[derive(Debug, Clone)]
pub struct AtlasBuilderConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub mip_level: u32,
    pub padding: u32,
}

impl Default for AtlasBuilderConfig {
    fn default() -> Self {
        Self {
            max_width: 4096,
            max_height: 4096,
            mip_level: 0,
            padding: 0,
        }
    }
}

/// A complete baked atlas sheet (chunk) with all active PBR buffers.
#[derive(Debug, Clone)]
pub struct BakedAtlasChunk {
    pub chunk_id: u16,
    pub category: String,
    pub is_animated: bool,
    pub category_chunk_index: usize,
    pub width: u32,
    pub height: u32,
    pub albedo: RgbaBuffer,
    /// Raw LabPBR `_n` companion (legacy in-memory access; not written to new caches).
    pub normal: Option<RgbaBuffer>,
    /// Raw LabPBR `_s` companion (legacy in-memory access; not written to new caches).
    pub specular: Option<RgbaBuffer>,
    pub overlay: Option<RgbaBuffer>,
    /// Decoded OpenGL normal RGB + height A (`MtkPackedV1`).
    pub normal_gl: Option<RgbaBuffer>,
    /// Decoded AO / roughness / metallic / emission mask (`MtkPackedV1`).
    pub orm: Option<RgbaBuffer>,
    /// Decoded IOR / porosity / SSS / transmission mask (`MtkPackedV1`).
    pub aux: Option<RgbaBuffer>,
}

impl BakedAtlasChunk {
    /// Canonical file stem for this atlas sheet, e.g. `"blocks_chunk_001"` or `"blocks_anim_chunk_001"`.
    pub fn file_stem(&self) -> String {
        if self.is_animated {
            format!(
                "{}_anim_chunk_{:03}",
                self.category, self.category_chunk_index
            )
        } else {
            format!("{}_chunk_{:03}", self.category, self.category_chunk_index)
        }
    }
}

/// Complete output containing all baked atlas sheets and authoritative address table.
#[derive(Debug, Clone)]
pub struct BakedAtlas {
    pub chunks: Vec<BakedAtlasChunk>,
    pub address_map: AtlasAddressMap,
}

impl BakedAtlas {
    /// Total number of baked atlas sheets / chunks.
    pub fn chunk_count(&self) -> usize {
        if !self.chunks.is_empty() {
            self.chunks.len()
        } else {
            self.address_map.chunks.len()
        }
    }

    /// Lookup chunk descriptor from address map by chunk ID.
    pub fn get_chunk_meta(
        &self,
        chunk_id: u16,
    ) -> Option<&crate::atlas::address_map::AtlasChunkMeta> {
        self.address_map.get_chunk_meta(chunk_id)
    }
}

/// Top-level coordinator for building vanilla Minecraft atlases with PBR sync.
pub struct AtlasBuilder {
    pub(crate) config: AtlasBuilderConfig,
}
