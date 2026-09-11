use serde::{Deserialize, Serialize};

/// Supported external DCC / Minecraft model importer origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ImporterOrigin {
    #[default]
    Auto,
    Mineways,
    Jmc2Obj,
    IceCube,
    Generic,
}

impl ImporterOrigin {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "mineways" => Self::Mineways,
            "jmc2obj" | "jmc" => Self::Jmc2Obj,
            "icecube" | "ice_cube" => Self::IceCube,
            "generic" => Self::Generic,
            _ => Self::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Mineways => "mineways",
            Self::Jmc2Obj => "jmc2obj",
            Self::IceCube => "ice_cube",
            Self::Generic => "generic",
        }
    }
}

/// Incoming source coordinate space for UV decoding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceUvSpace {
    /// Normalized [0.0..1.0] (or tiled [0.0..N.0]) local texture space.
    Local,
    /// Mineways merged terrain atlas with texture pixel dimensions.
    MinewaysAtlas { width: u32, height: u32 },
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
