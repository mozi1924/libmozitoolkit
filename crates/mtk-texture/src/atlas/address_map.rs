use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use mtk_resource::{AnimationMetadata, ResourceLocation};

/// Classification of sprite placement and usage mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpriteKind {
    /// Placed on a static chunk (either a truly static sprite or Frame 0 of an animated sprite).
    StaticAtlas,
    /// Placed on a dedicated animated chunk (full vertical strip, frame_count > 1).
    AnimatedAtlas,
    /// Standalone-only texture.
    StandaloneOnly,
}

impl Default for SpriteKind {
    fn default() -> Self {
        Self::StaticAtlas
    }
}

/// Address and metrics for a single sprite located within a stitched Atlas Chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasSpriteLocation {
    pub chunk_id: u16,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub is_animated: bool,
    #[serde(default)]
    pub sprite_kind: SpriteKind,
    pub texture_id: u32,
    /// Normalized UV bounds: `[u_min, v_min, u_max, v_max]` in [0.0..1.0] atlas space (Frame 0 or static frame).
    pub uv_bounds: [f32; 4],
    /// Normalized UV bounds of Frame 0 specifically: `[u_min, v_min, u_max, v_max]`.
    #[serde(default)]
    pub frame_0_uv_bounds: [f32; 4],
    /// Normalized local UV bounds in single-block [0.0..1.0] space (default [0.0, 0.0, 1.0, 1.0]).
    #[serde(default = "default_local_uv_bounds")]
    pub local_uv_bounds: [f32; 4],
    /// Step size in UV space per animation frame: `[u_step, v_step]`.
    #[serde(default)]
    pub frame_uv_step: [f32; 2],
    /// Physical pixel rectangle on the atlas chunk for Frame 0: `[x, y, width, height]`.
    pub pixel_rect: [u32; 4],
    /// Physical pixel rectangle on the atlas chunk for the entire animation strip: `[x, y, width, height]`.
    #[serde(default)]
    pub strip_pixel_rect: [u32; 4],
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
    /// Whether this slot contains a non-default Overlay companion map.
    #[serde(default)]
    pub has_overlay: bool,
}

fn default_category() -> String {
    "blocks".to_string()
}

fn default_local_uv_bounds() -> [f32; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

/// Metadata for an individual Atlas Chunk (sheet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtlasChunkMeta {
    pub chunk_id: u16,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub is_animated: bool,
    #[serde(default = "default_chunk_index")]
    pub category_chunk_index: usize,
    pub width: u32,
    pub height: u32,
    pub has_normal: bool,
    pub has_specular: bool,
    #[serde(default)]
    pub has_overlay: bool,
}

fn default_chunk_index() -> usize {
    1
}

impl AtlasChunkMeta {
    /// Canonical file stem for this atlas sheet, e.g. `"blocks_chunk_001"` or `"blocks_anim_chunk_001"`.
    pub fn file_stem(&self) -> String {
        if self.is_animated {
            format!("{}_anim_chunk_{:03}", self.category, self.category_chunk_index)
        } else {
            format!("{}_chunk_{:03}", self.category, self.category_chunk_index)
        }
    }
}

/// Authoritative mapping table holding all sprite locations and chunk descriptors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AtlasAddressMap {
    pub chunks: Vec<AtlasChunkMeta>,
    /// Static atlas sprites (100% texture coverage, Frame 0 for animated sprites).
    pub sprites: HashMap<ResourceLocation, AtlasSpriteLocation>,
    /// Dedicated animated atlas sprites (full strips for multi-frame animated textures).
    #[serde(default)]
    pub anim_sprites: HashMap<ResourceLocation, AtlasSpriteLocation>,
}

impl AtlasAddressMap {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            sprites: HashMap::new(),
            anim_sprites: HashMap::new(),
        }
    }

    /// O(1) primary lookup by exact ResourceLocation (checks static sprites first, then anim sprites).
    pub fn lookup(&self, location: &ResourceLocation) -> Option<&AtlasSpriteLocation> {
        self.sprites.get(location).or_else(|| self.anim_sprites.get(location))
    }

    /// Lookup static Frame 0 sprite location (100% coverage across all textures).
    pub fn lookup_static(&self, location: &ResourceLocation) -> Option<&AtlasSpriteLocation> {
        self.sprites.get(location)
    }

    /// Lookup dedicated animated strip sprite location.
    pub fn lookup_animated(&self, location: &ResourceLocation) -> Option<&AtlasSpriteLocation> {
        self.anim_sprites.get(location)
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

    /// Lookup static Frame 0 sprite location by string.
    pub fn lookup_static_str(&self, s: &str) -> Option<&AtlasSpriteLocation> {
        if let Ok(loc) = ResourceLocation::parse_texture_path(s) {
            self.lookup_static(&loc)
        } else if let Ok(loc) = ResourceLocation::parse(s) {
            self.lookup_static(&loc)
        } else {
            None
        }
    }

    /// Lookup animated strip sprite location by string.
    pub fn lookup_animated_str(&self, s: &str) -> Option<&AtlasSpriteLocation> {
        if let Ok(loc) = ResourceLocation::parse_texture_path(s) {
            self.lookup_animated(&loc)
        } else if let Ok(loc) = ResourceLocation::parse(s) {
            self.lookup_animated(&loc)
        } else {
            None
        }
    }

    /// Merge another address map into this one.
    pub fn merge(&mut self, other: AtlasAddressMap) {
        self.chunks.extend(other.chunks);
        self.sprites.extend(other.sprites);
        self.anim_sprites.extend(other.anim_sprites);
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
