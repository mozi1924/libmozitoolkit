use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Packed UV transform [scale_u, scale_v, offset_u, offset_v] for each face.
    pub face_uv_transforms: Vec<[f32; 4]>,
    /// UV rotation angle in radians for each face (Euler Z).
    pub face_uv_rotations: Vec<f32>,
    /// Whether each face targets a dedicated animated atlas chunk.
    pub face_is_animated: Vec<bool>,
    /// Per-face animation timing `[total_frames, frametime, interpolate]`.
    pub face_anim_timing: Vec<[f32; 3]>,
    /// Per-face animation frame size `[frame_width, frame_height, 0]`.
    pub face_anim_frame_size: Vec<[f32; 3]>,
    /// Number of faces with unresolved textures.
    pub unmapped_faces: usize,
}

/// Paletted remap result: mesh UV data plus compact source-texture provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshPalettedUvRemapResult {
    /// Underlying per-face UV/material routing result.
    pub mesh: MeshMultiUvRemapResult,
    /// Unique source texture keys (atlas keys + valid legacy provenance fallbacks).
    pub source_key_palette: Vec<String>,
    /// Per-face index into `source_key_palette`.
    pub face_source_key_indices: Vec<u32>,
}

/// Resolved atlas routing for a single face, independent of UV remapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshMaterialRoute {
    /// Target Atlas Chunk ID (static or animated).
    pub chunk_id: u16,
    /// Target Texture ID.
    pub texture_id: u32,
    /// Whether the target sprite lives in a dedicated animated chunk.
    pub is_animated: bool,
    /// Animation timing `[total_frames, frametime, interpolate]`.
    pub anim_timing: [f32; 3],
    /// Animation frame size `[frame_width, frame_height, 0]`.
    pub anim_frame_size: [f32; 3],
    /// Whether the material name was resolved to an atlas sprite.
    pub mapped: bool,
}

/// Result of re-routing an already atlas-mapped mesh to a new animation policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshAnimationRerouteResult {
    /// Atlas UVs re-projected from the previous sprite to the new sprite.
    pub atlas_uvs: Vec<[f32; 2]>,
    /// Target Atlas Chunk ID for each face.
    pub face_chunk_ids: Vec<u16>,
    /// Target Texture ID for each face.
    pub face_texture_ids: Vec<u32>,
    /// Whether each face targets an animated chunk.
    pub face_is_animated: Vec<bool>,
    /// Per-face animation timing.
    pub face_anim_timing: Vec<[f32; 3]>,
    /// Per-face animation frame size.
    pub face_anim_frame_size: Vec<[f32; 3]>,
    /// Faces whose material could not be resolved.
    pub unmapped_faces: usize,
    /// Faces whose previous atlas sprite could not be located, so UVs were kept.
    pub uv_unmapped_faces: usize,
}
