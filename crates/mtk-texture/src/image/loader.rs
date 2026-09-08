use mtk_resource::{AnimationMetadata, DiscoveredSprite, ResourceLocation};
use crate::error::TextureError;
use crate::image::buffer::RgbaBuffer;

/// A fully decoded sprite with single-frame metrics and optional PBR companions.
#[derive(Debug, Clone)]
pub struct DecodedSprite {
    pub sprite_id: ResourceLocation,
    pub albedo: RgbaBuffer,
    pub normal: Option<RgbaBuffer>,
    pub specular: Option<RgbaBuffer>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub frame_count: u32,
    pub metadata: Option<AnimationMetadata>,
}

impl DecodedSprite {
    /// Decode a discovered sprite entry from its raw PNG byte buffers.
    pub fn from_discovered(discovered: DiscoveredSprite) -> Result<Self, TextureError> {
        let raw_albedo = discovered.raw_albedo.ok_or_else(|| {
            TextureError::Baking(format!("Missing albedo bytes for sprite '{}'", discovered.sprite_id))
        })?;

        let albedo = RgbaBuffer::from_png_bytes(&raw_albedo)?;
        let normal = match discovered.raw_normal {
            Some(ref bytes) => RgbaBuffer::from_png_bytes(bytes).ok(),
            None => None,
        };
        let specular = match discovered.raw_specular {
            Some(ref bytes) => RgbaBuffer::from_png_bytes(bytes).ok(),
            None => None,
        };

        // Determine single-frame dimensions:
        // Vanilla standard: ONLY textures with AnimationMetadata (.png.mcmeta with "animation": { ... })
        // are multi-frame animations. If metadata is present, default frame_height to albedo.width
        // (if height >= width) or albedo.height.
        // If NO AnimationMetadata is present, the texture is static (regardless of aspect ratio:
        // paintings e.g. 16x32, 48x64; entities e.g. 64x128, 16x256; blocks; GUI, etc.).
        let (frame_width, frame_height, frame_count) = if let Some(ref meta) = discovered.metadata {
            let fw = meta.width.unwrap_or(albedo.width);
            let fh = meta.height.unwrap_or_else(|| {
                if albedo.height >= albedo.width && albedo.width > 0 {
                    albedo.width
                } else {
                    albedo.height
                }
            });
            let fc = if fh > 0 { albedo.height / fh } else { 1 };
            (fw, fh, fc.max(1))
        } else {
            (albedo.width, albedo.height, 1)
        };

        Ok(Self {
            sprite_id: discovered.sprite_id,
            albedo,
            normal,
            specular,
            frame_width,
            frame_height,
            frame_count,
            metadata: discovered.metadata,
        })
    }

    /// Parallel or sequential batch decode of discovered sprites.
    pub fn decode_batch(sprites: Vec<DiscoveredSprite>) -> Result<Vec<DecodedSprite>, TextureError> {
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            sprites
                .into_par_iter()
                .map(Self::from_discovered)
                .collect()
        }

        #[cfg(not(feature = "parallel"))]
        {
            sprites.into_iter().map(Self::from_discovered).collect()
        }
    }
}
