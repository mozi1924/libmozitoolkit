#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Identifies biome or model tinting behavior.
/// -1 represents no tint. >= 0 references tint index in Minecraft model element face.
pub type TintIndex = i16;

/// Material slot identifier, mapped to shader material or atlas slice.
pub type MaterialSlotId = u16;

/// Light level encoding block light (0..15) and sky light (0..15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LightLevel {
    pub block: u8,
    pub sky: u8,
}

impl LightLevel {
    pub const ZERO: Self = Self { block: 0, sky: 0 };
    pub const MAX: Self = Self { block: 15, sky: 15 };

    #[inline]
    pub const fn new(block: u8, sky: u8) -> Self {
        Self {
            block: if block > 15 { 15 } else { block },
            sky: if sky > 15 { 15 } else { sky },
        }
    }

    #[inline]
    pub const fn pack_u8(self) -> u8 {
        (self.block & 0x0F) | ((self.sky & 0x0F) << 4)
    }

    #[inline]
    pub const fn unpack_u8(packed: u8) -> Self {
        Self {
            block: packed & 0x0F,
            sky: (packed >> 4) & 0x0F,
        }
    }
}

/// Standard geometric attribute domain corresponding to modern DCC & USD standards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum AttributeDomain {
    /// Per-vertex / point data (size = vertex_count, e.g. positions, normals, weights).
    #[default]
    Point,
    /// Per-face-corner / loop data (size = loop_count / indices.len(), e.g. UVs, corner colors).
    Corner,
    /// Per-face / polygon data (size = face_count, e.g. material_slot, chunk_id, tint_index, texture_key).
    Face,
    /// Constant / mesh-level global data (size = 1).
    Mesh,
}

impl AttributeDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::Corner => "corner",
            Self::Face => "face",
            Self::Mesh => "mesh",
        }
    }
}

/// Dynamic, contiguous typed attribute storage for zero-copy memory operations.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AttributeData {
    Float(Vec<f32>),
    Float2(Vec<[f32; 2]>),
    Float3(Vec<[f32; 3]>),
    Float4(Vec<[f32; 4]>),
    Int8(Vec<i8>),
    Int16(Vec<i16>),
    Int32(Vec<i32>),
    UInt8(Vec<u8>),
    UInt16(Vec<u16>),
    UInt32(Vec<u32>),
    String(Vec<String>),
    Bool(Vec<bool>),
}

impl AttributeData {
    /// Number of elements in this attribute array.
    pub fn len(&self) -> usize {
        match self {
            Self::Float(v) => v.len(),
            Self::Float2(v) => v.len(),
            Self::Float3(v) => v.len(),
            Self::Float4(v) => v.len(),
            Self::Int8(v) => v.len(),
            Self::Int16(v) => v.len(),
            Self::Int32(v) => v.len(),
            Self::UInt8(v) => v.len(),
            Self::UInt16(v) => v.len(),
            Self::UInt32(v) => v.len(),
            Self::String(v) => v.len(),
            Self::Bool(v) => v.len(),
        }
    }

    /// Whether this attribute array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a string identifier of the data type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Float(_) => "float",
            Self::Float2(_) => "float2",
            Self::Float3(_) => "float3",
            Self::Float4(_) => "float4",
            Self::Int8(_) => "int8",
            Self::Int16(_) => "int16",
            Self::Int32(_) => "int32",
            Self::UInt8(_) => "uint8",
            Self::UInt16(_) => "uint16",
            Self::UInt32(_) => "uint32",
            Self::String(_) => "string",
            Self::Bool(_) => "bool",
        }
    }

    /// Read-only slice of raw underlying contiguous bytes (None for non-POD types like String).
    pub fn as_bytes(&self) -> Option<&[u8]> {
        unsafe {
            match self {
                Self::Float(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<f32>(),
                )),
                Self::Float2(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<[f32; 2]>(),
                )),
                Self::Float3(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<[f32; 3]>(),
                )),
                Self::Float4(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<[f32; 4]>(),
                )),
                Self::Int8(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<i8>(),
                )),
                Self::Int16(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<i16>(),
                )),
                Self::Int32(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<i32>(),
                )),
                Self::UInt8(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<u8>(),
                )),
                Self::UInt16(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<u16>(),
                )),
                Self::UInt32(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<u32>(),
                )),
                Self::Bool(v) => Some(std::slice::from_raw_parts(
                    v.as_ptr() as *const u8,
                    v.len() * std::mem::size_of::<bool>(),
                )),
                Self::String(_) => None,
            }
        }
    }
}

/// Generic mesh attribute definition.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MeshAttribute {
    pub name: String,
    pub domain: AttributeDomain,
    pub data: AttributeData,
}

impl MeshAttribute {
    pub fn new(name: impl Into<String>, domain: AttributeDomain, data: AttributeData) -> Self {
        Self {
            name: name.into(),
            domain,
            data,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.data.as_bytes()
    }
}

/// Canonical built-in mesh attribute names (libmtk Spec v1).
pub mod constants {
    /// Canonical source texture identifier (Domain: Face, Type: String).
    pub const ATTR_SOURCE_TEXTURE: &str = "mtk_source_texture";
    /// Backwards-compatible alias for source texture key.
    pub const ATTR_SOURCE_TEXTURE_KEY: &str = "mtk_source_texture_key";
    /// Source provenance file or model origin path.
    pub const ATTR_SOURCE_ORIGIN: &str = "mtk_source_origin";

    /// Target Atlas chunk/sheet slot index (Domain: Face, Type: Int32/UInt16).
    pub const ATTR_ATLAS_CHUNK_ID: &str = "mtk_atlas_chunk_id";
    /// Target Atlas texture/sprite index (Domain: Face, Type: UInt32).
    pub const ATTR_ATLAS_TEXTURE_ID: &str = "mtk_atlas_texture_id";
    /// Material slot ID assigned to the polygon.
    pub const ATTR_MATERIAL_SLOT: &str = "mtk_material_slot";
    pub const ATTR_MATERIAL_ID: &str = "mtk_material_id";

    /// Scalar emission intensity (Domain: Face/Point, Type: Float32, range 0.0..15.0 or 0.0..1.0).
    pub const ATTR_EMISSION: &str = "mtk_emission";
    /// Packed physical properties (Domain: Face, Type: Float4/RGBA [emission, roughness_mult, metallic, thin_wall]).
    pub const ATTR_MATERIAL_PROPS: &str = "mtk_material_props";

    /// Packed UV affine transform (Domain: Face, Type: Float4/RGBA [scale_u, scale_v, offset_u, offset_v]).
    pub const ATTR_UV_TRANSFORM: &str = "mtk_uv_transform";
    /// Dynamic UV rotation angle in radians (Domain: Face, Type: Float32).
    pub const ATTR_UV_ROTATION: &str = "mtk_uv_rotation";
    /// UV decoder routing mode (Domain: Face, Type: UInt8/Int32: 0=Atlas, 1=Standalone, 2=Animated, 3=Overlay).
    pub const ATTR_UV_MODE: &str = "mtk_uv_mode";

    /// Animation timing parameters (Domain: Face, Type: Float3 [total_frames, frametime, interpolate]).
    pub const ATTR_ANIM_TIMING: &str = "mtk_anim_timing";
    /// Animation frame size (Domain: Face, Type: Float3 [frame_width, frame_height, 0]).
    pub const ATTR_ANIM_FRAME_SIZE: &str = "mtk_anim_frame_size";

    /// Resolved linear RGBA biome tint color (Domain: Face/Corner, Type: Float4).
    pub const ATTR_BIOME_TINT_COLOR: &str = "mtk_biome_tint_color";
    /// Packed biome tint data and weights (Domain: Face, Type: Float4 [base_weight, overlay_weight, tint_weight, tint_type]).
    pub const ATTR_BIOME_TINT_DATA: &str = "mtk_biome_tint_data";
    /// Biome colormap UV coordinate (Domain: Face, Type: Float3 [u, v, 0]).
    pub const ATTR_COLORMAP_UV: &str = "mtk_colormap_uv";

    /// Minecraft absolute world coordinates (Domain: Face/Point, Type: Int32 x 3 or Float3).
    pub const ATTR_BLOCK_POS: &str = "mtk_block_pos";
    pub const ATTR_BLOCK_X: &str = "mtk_block_x";
    pub const ATTR_BLOCK_Y: &str = "mtk_block_y";
    pub const ATTR_BLOCK_Z: &str = "mtk_block_z";

    /// Canonical 6-way face normal direction (Domain: Face, Type: UInt8/Int32: 0=Down, 1=Up, 2=North, 3=South, 4=West, 5=East).
    pub const ATTR_FACE_DIR: &str = "mtk_face_dir";
    /// Light level (Domain: Face/Point, Type: UInt8 or Float2 [block_light, sky_light]).
    pub const ATTR_LIGHT_LEVEL: &str = "mtk_light_level";
    /// Whether the face is opaque (Domain: Face, Type: Bool).
    pub const ATTR_IS_OPAQUE: &str = "mtk_is_opaque";
}

/// Face-level metadata attached to baked geometry.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FaceAttributes {
    /// Texture identifier or atlas sprite key.
    pub texture_key: String,
    /// Material slot index.
    pub material_slot: MaterialSlotId,
    /// Tint index for colormap or biome blending (-1 for none).
    pub tint_index: TintIndex,
    /// Baked emission intensity (0.0 .. 15.0 or 0.0 .. 1.0).
    pub emission: f32,
    /// Whether this face is an overlay layer (e.g. grass side overlay).
    pub is_overlay: bool,
    /// UV routing mode: 0 = Atlas, 1 = Standalone Static, 2 = Standalone Anim, 3 = Overlay Local.
    pub uv_mode: u8,
    /// Atlas chunk/tile ID if mapped into a global atlas.
    pub atlas_chunk_id: Option<u32>,
    /// Atlas texture/sprite ID within chunk.
    pub atlas_texture_id: Option<u32>,
    /// Packed UV affine transform [scale_u, scale_v, offset_u, offset_v].
    pub uv_transform: [f32; 4],
    /// UV rotation angle in radians.
    pub uv_rotation: f32,
    /// Canonical 6-way face normal direction (0..5).
    pub face_dir: u8,
    /// Packed physical properties [emission, roughness_mult, metallic, thin_wall].
    pub material_props: [f32; 4],
    /// Animation timing [total_frames, frametime, interpolate].
    pub anim_timing: [f32; 3],
    /// Animation frame size [frame_width, frame_height, 0].
    pub anim_frame_size: [f32; 3],
    /// Resolved linear RGBA tint color [r, g, b, a].
    pub biome_tint_color: [f32; 4],
    /// Packed biome tint data [base_weight, overlay_weight, tint_weight, tint_type].
    pub biome_tint_data: [f32; 4],
    /// Colormap UV sampling coordinate [u, v, 0].
    pub colormap_uv: [f32; 3],
    /// Minecraft absolute block coordinate [x, y, z].
    pub block_pos: [i32; 3],
    /// Light level.
    pub light_level: LightLevel,
}

impl Default for FaceAttributes {
    fn default() -> Self {
        Self {
            texture_key: String::new(),
            material_slot: 0,
            tint_index: -1,
            emission: 0.0,
            is_overlay: false,
            uv_mode: 0,
            atlas_chunk_id: None,
            atlas_texture_id: None,
            uv_transform: [1.0, 1.0, 0.0, 0.0],
            uv_rotation: 0.0,
            face_dir: 1, // Default Up
            material_props: [0.0, 1.0, 0.0, 0.0],
            anim_timing: [1.0, 1.0, 0.0],
            anim_frame_size: [16.0, 16.0, 0.0],
            biome_tint_color: [1.0, 1.0, 1.0, 1.0],
            biome_tint_data: [1.0, 1.0, 0.0, 0.0],
            colormap_uv: [0.0, 0.0, 0.0],
            block_pos: [0, 0, 0],
            light_level: LightLevel::ZERO,
        }
    }
}

impl FaceAttributes {
    /// Creates a new `FaceAttributes` with given texture key and material slot.
    pub fn new(texture_key: impl Into<String>, material_slot: MaterialSlotId) -> Self {
        Self {
            texture_key: texture_key.into(),
            material_slot,
            ..Default::default()
        }
    }

    /// Sets emission strength.
    pub fn with_emission(mut self, emission: f32) -> Self {
        self.emission = emission;
        self.material_props[0] = emission;
        self
    }

    /// Sets UV affine transform.
    pub fn with_uv_transform(mut self, scale_u: f32, scale_v: f32, offset_u: f32, offset_v: f32) -> Self {
        self.uv_transform = [scale_u, scale_v, offset_u, offset_v];
        self
    }

    /// Sets biome tint parameters.
    pub fn with_biome_tint(mut self, color: [f32; 4], data: [f32; 4], colormap_uv: [f32; 3]) -> Self {
        self.biome_tint_color = color;
        self.biome_tint_data = data;
        self.colormap_uv = colormap_uv;
        self
    }
}
