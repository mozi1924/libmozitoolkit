use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Generic specification for decoding grid-based texture atlases (e.g. Mineways-style atlases).
/// This completely decouples libmtk from any specific DCC exporter format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridAtlasSpec {
    /// Total width and height of single swatch cell in pixels (e.g. 18.0 for 16px tile + 2px border).
    pub swatch_size: f32,
    /// Usable tile dimensions in pixels (e.g. 16.0).
    pub tile_size: f32,
    /// Inset border margin in pixels (e.g. 1.0).
    pub border: f32,
    /// Image width in pixels.
    pub image_width: u32,
    /// Image height in pixels.
    pub image_height: u32,
    /// Filename patterns/prefixes used to detect this atlas (e.g. ["terrain", "terrainrgba", "terrainrgb"]).
    pub atlas_name_patterns: Vec<String>,
    /// Filename suffixes used to detect this atlas (e.g. ["_rgb", "_rgba", "_alpha"]).
    pub atlas_suffix_patterns: Vec<String>,
    /// Mapping from swatch ID (col + row * cols_per_row) to candidate texture identifiers.
    pub swatch_to_candidates: HashMap<usize, Vec<String>>,
}

impl Default for GridAtlasSpec {
    fn default() -> Self {
        Self {
            swatch_size: 18.0,
            tile_size: 16.0,
            border: 1.0,
            image_width: 1024,
            image_height: 1024,
            atlas_name_patterns: Vec::new(),
            atlas_suffix_patterns: Vec::new(),
            swatch_to_candidates: HashMap::new(),
        }
    }
}

impl GridAtlasSpec {
    /// Check if a given texture/material name matches this grid atlas specification.
    pub fn matches_atlas_name(&self, raw_name: &str) -> bool {
        let clean = raw_name.trim().to_lowercase();
        let stem = clean
            .strip_suffix(".png")
            .or_else(|| clean.strip_suffix(".jpg"))
            .unwrap_or(&clean);

        // Strip blender numerical duplicate suffixes (e.g. ".001", "_001")
        let stem = if let Some(idx) = stem.rfind('.') {
            if stem[idx + 1..].chars().all(|c| c.is_ascii_digit()) {
                &stem[..idx]
            } else {
                stem
            }
        } else {
            stem
        };

        for pat in &self.atlas_name_patterns {
            let pat_lower = pat.to_lowercase();
            if stem == pat_lower || stem.starts_with(&pat_lower) {
                return true;
            }
        }

        for suf in &self.atlas_suffix_patterns {
            let suf_lower = suf.to_lowercase();
            if stem.ends_with(&suf_lower) {
                return true;
            }
        }

        false
    }
}

/// Incoming source coordinate space for UV decoding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceUvSpace {
    /// Normalized [0.0..1.0] (or tiled [0.0..N.0]) local texture space.
    Local,
    /// Grid-based atlas with texture pixel dimensions.
    GridAtlas { width: u32, height: u32 },
    /// Existing Atlas Chunk with normalized UV bounds [u_min, v_min, u_max, v_max].
    AtlasSprite { bounds: [f32; 4] },
}

/// Summary result of a batch mesh UV remapping operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshRemapResult {
    /// Number of faces processed.
    pub face_count: usize,
    /// Number of UV loops remapped.
    pub loop_count: usize,
    /// Target Atlas Chunk ID for each face.
    pub face_chunk_ids: Vec<u16>,
    /// Target Texture ID for each face.
    pub face_texture_ids: Vec<u32>,
    /// Number of faces with unresolved textures.
    pub unmapped_faces: usize,
}

/// Summary result of a multi-UV batch mesh remapping operation (Atlas UV + Local UV).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshMultiUvRemapResult {
    /// Number of faces processed.
    pub face_count: usize,
    /// Number of UV loops processed.
    pub loop_count: usize,
    /// Primary Atlas UVs mapped to chunk coordinates: [[u, v], ...]
    pub atlas_uvs: Vec<[f32; 2]>,
    /// Normalized Standalone/Local UVs [0..1]: [[u, v], ...]
    pub local_uvs: Vec<[f32; 2]>,
    /// Target Atlas Chunk ID for each face.
    pub face_chunk_ids: Vec<u16>,
    /// Target Texture ID for each face.
    pub face_texture_ids: Vec<u32>,
    /// Face UV routing mode (0 = Atlas, 1 = Standalone Static, 2 = Standalone Anim, 3 = Overlay Local).
    pub face_uv_modes: Vec<u8>,
    /// Whether each face is an overlay layer (e.g. grass side overlay).
    pub face_is_overlay: Vec<bool>,
    /// Number of faces with unresolved textures.
    pub unmapped_faces: usize,
}
