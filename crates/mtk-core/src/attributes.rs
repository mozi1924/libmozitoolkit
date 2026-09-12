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
    /// Baked emission intensity (0.0 .. 1.0 or light emission tier).
    pub emission: f32,
    /// Whether this face is an overlay layer (e.g. grass side overlay).
    pub is_overlay: bool,
    /// UV routing mode: 0 = Atlas, 1 = Standalone Static, 2 = Standalone Anim, 3 = Overlay Local.
    pub uv_mode: u8,
    /// Atlas chunk/tile ID if mapped into a global atlas.
    pub atlas_chunk_id: Option<u32>,
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
        }
    }
}
