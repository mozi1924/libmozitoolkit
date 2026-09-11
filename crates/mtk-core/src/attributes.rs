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
