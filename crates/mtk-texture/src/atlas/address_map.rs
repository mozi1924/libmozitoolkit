use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use mtk_resource::{AnimationMetadata, ResourceLocation};

/// Address and metrics for a single sprite located within a stitched Atlas Chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasSpriteLocation {
    pub chunk_id: u16,
    #[serde(default = "default_category")]
    pub category: String,
    pub texture_id: u32,
    /// Normalized UV bounds: `[u_min, v_min, u_max, v_max]` in [0.0..1.0] atlas space.
    pub uv_bounds: [f32; 4],
    /// Physical pixel rectangle on the atlas chunk: `[x, y, width, height]`.
    pub pixel_rect: [u32; 4],
    /// Single-frame physical resolution: `[frame_width, frame_height]`.
    pub frame_size: [u32; 2],
    /// Total animation frames.
    pub frame_count: u32,
    /// Optional animation timing metadata.
    pub animation: Option<AnimationMetadata>,
    /// Whether this slot contains a non-default Normal companion map.
    pub has_normal: bool,
    /// Whether this slot contains a non-default Specular companion map.
    pub has_specular: bool,
}

fn default_category() -> String {
    "blocks".to_string()
}

/// Metadata for an individual Atlas Chunk (sheet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasChunkMeta {
    pub chunk_id: u16,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_chunk_index")]
    pub category_chunk_index: usize,
    pub width: u32,
    pub height: u32,
    pub has_normal: bool,
    pub has_specular: bool,
}

fn default_chunk_index() -> usize {
    1
}

impl AtlasChunkMeta {
    /// Canonical file stem for this atlas sheet, e.g. `"blocks_chunk_001"`.
    pub fn file_stem(&self) -> String {
        format!("{}_chunk_{:03}", self.category, self.category_chunk_index)
    }
}

/// Authoritative mapping table holding all sprite locations and chunk descriptors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AtlasAddressMap {
    pub chunks: Vec<AtlasChunkMeta>,
    pub sprites: HashMap<ResourceLocation, AtlasSpriteLocation>,
}

impl AtlasAddressMap {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            sprites: HashMap::new(),
        }
    }

    /// O(1) lookup by exact ResourceLocation.
    pub fn lookup(&self, location: &ResourceLocation) -> Option<&AtlasSpriteLocation> {
        self.sprites.get(location)
    }

    /// O(1) lookup by string (e.g. `"minecraft:block/stone"`, `"block/stone"`, or `"textures/block/stone.png"`).
    pub fn lookup_str(&self, s: &str) -> Option<&AtlasSpriteLocation> {
        if let Ok(loc) = ResourceLocation::parse_texture_path(s) {
            self.lookup(&loc)
        } else if let Ok(loc) = ResourceLocation::parse(s) {
            self.lookup(&loc)
        } else {
            None
        }
    }

    /// Merge another address map into this one.
    pub fn merge(&mut self, other: AtlasAddressMap) {
        self.chunks.extend(other.chunks);
        self.sprites.extend(other.sprites);
    }

    /// Serialize to formatted JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}
