//! # Authoritative Vanilla Colormap Sampling & Memory Cache
//!
//! Provides in-memory resident Minecraft 256x256 colormaps (grass, foliage, dry_foliage)
//! for nanosecond-speed lookup, resolving linear RGBA tint colors in Rust without
//! requiring Blender shader node tree texture sampling.

use std::sync::OnceLock;

use super::palettes::{sample_colormap_pixel, srgb_to_linear};

const GRASS_PNG: &[u8] = include_bytes!("../../assets/colormap/grass.png");
const FOLIAGE_PNG: &[u8] = include_bytes!("../../assets/colormap/foliage.png");
const DRY_FOLIAGE_PNG: &[u8] = include_bytes!("../../assets/colormap/dry_foliage.png");

/// Resident in-memory 256x256 RGB colormap buffers.
pub struct VanillaColormaps {
    pub grass_pixels: Vec<u8>,
    pub foliage_pixels: Vec<u8>,
    pub dry_foliage_pixels: Vec<u8>,
}

impl VanillaColormaps {
    /// Loads and decodes embedded canonical vanilla colormaps into uncompressed raw RGB bytes.
    fn load_embedded() -> Self {
        let decode = |bytes: &[u8], name: &str| -> Vec<u8> {
            match image::load_from_memory_with_format(bytes, image::ImageFormat::Png) {
                Ok(img) => img.to_rgb8().into_raw(),
                Err(e) => {
                    // Fallback to 256x256 white if decoding fails for any reason
                    eprintln!(
                        "Warning: failed to decode embedded colormap {}: {}",
                        name, e
                    );
                    vec![255u8; 256 * 256 * 3]
                }
            }
        };

        Self {
            grass_pixels: decode(GRASS_PNG, "grass.png"),
            foliage_pixels: decode(FOLIAGE_PNG, "foliage.png"),
            dry_foliage_pixels: decode(DRY_FOLIAGE_PNG, "dry_foliage.png"),
        }
    }

    /// Sample grass sRGB color [0.0..1.0] from temperature and downfall.
    #[inline]
    pub fn sample_grass_srgb(&self, temperature: f32, downfall: f32) -> [f32; 3] {
        sample_colormap_pixel(&self.grass_pixels, 256, 256, temperature, downfall, 3)
    }

    /// Sample foliage sRGB color [0.0..1.0] from temperature and downfall.
    #[inline]
    pub fn sample_foliage_srgb(&self, temperature: f32, downfall: f32) -> [f32; 3] {
        sample_colormap_pixel(&self.foliage_pixels, 256, 256, temperature, downfall, 3)
    }

    /// Sample dry foliage sRGB color [0.0..1.0] from temperature and downfall.
    #[inline]
    pub fn sample_dry_foliage_srgb(&self, temperature: f32, downfall: f32) -> [f32; 3] {
        sample_colormap_pixel(&self.dry_foliage_pixels, 256, 256, temperature, downfall, 3)
    }

    /// Sample grass color directly as Linear RGBA [r, g, b, 1.0].
    #[inline]
    pub fn sample_grass_linear(&self, temperature: f32, downfall: f32) -> [f32; 4] {
        let srgb = self.sample_grass_srgb(temperature, downfall);
        [
            srgb_to_linear(srgb[0]),
            srgb_to_linear(srgb[1]),
            srgb_to_linear(srgb[2]),
            1.0,
        ]
    }

    /// Sample foliage color directly as Linear RGBA [r, g, b, 1.0].
    #[inline]
    pub fn sample_foliage_linear(&self, temperature: f32, downfall: f32) -> [f32; 4] {
        let srgb = self.sample_foliage_srgb(temperature, downfall);
        [
            srgb_to_linear(srgb[0]),
            srgb_to_linear(srgb[1]),
            srgb_to_linear(srgb[2]),
            1.0,
        ]
    }

    /// Sample dry foliage color directly as Linear RGBA [r, g, b, 1.0].
    #[inline]
    pub fn sample_dry_foliage_linear(&self, temperature: f32, downfall: f32) -> [f32; 4] {
        let srgb = self.sample_dry_foliage_srgb(temperature, downfall);
        [
            srgb_to_linear(srgb[0]),
            srgb_to_linear(srgb[1]),
            srgb_to_linear(srgb[2]),
            1.0,
        ]
    }
}

static DEFAULT_COLORMAPS: OnceLock<VanillaColormaps> = OnceLock::new();

/// Returns the singleton instance of authoritative resident vanilla colormaps.
#[inline]
pub fn get_default_colormaps() -> &'static VanillaColormaps {
    DEFAULT_COLORMAPS.get_or_init(VanillaColormaps::load_embedded)
}

/// Applies the vanilla `GrassColorModifier.DARK_FOREST` to a base sRGB grass color:
/// `((baseColor & 0xFEFEFE) + 0x28340A) >> 1`
/// Returns the resulting Linear RGBA.
pub fn apply_dark_forest_grass_modifier(base_srgb: [f32; 3]) -> [f32; 4] {
    let r = (base_srgb[0] * 255.0).round().clamp(0.0, 255.0) as u32;
    let g = (base_srgb[1] * 255.0).round().clamp(0.0, 255.0) as u32;
    let b = (base_srgb[2] * 255.0).round().clamp(0.0, 255.0) as u32;

    let base_color = (r << 16) | (g << 8) | b;
    let modified = ((base_color & 0xFEFEFE) + 0x28340A) >> 1;

    let mr = ((modified >> 16) & 0xFF) as f32 / 255.0;
    let mg = ((modified >> 8) & 0xFF) as f32 / 255.0;
    let mb = (modified & 0xFF) as f32 / 255.0;

    [
        srgb_to_linear(mr),
        srgb_to_linear(mg),
        srgb_to_linear(mb),
        1.0,
    ]
}

/// Computes the authoritative linear RGBA tint for a given temperature, humidity, and category.
/// If `is_dark_forest` is true, applies the dark forest modifier to grass tint.
pub fn compute_resolved_tint_color(
    tint_type: u8,
    temperature: f32,
    humidity: f32,
    is_dark_forest: bool,
) -> [f32; 4] {
    let cms = get_default_colormaps();
    match tint_type {
        crate::biome::TINT_TYPE_GRASS => {
            if is_dark_forest {
                let base = cms.sample_grass_srgb(temperature, humidity);
                apply_dark_forest_grass_modifier(base)
            } else {
                cms.sample_grass_linear(temperature, humidity)
            }
        }
        crate::biome::TINT_TYPE_FOLIAGE => cms.sample_foliage_linear(temperature, humidity),
        crate::biome::TINT_TYPE_DRY_FOLIAGE => cms.sample_dry_foliage_linear(temperature, humidity),
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}
