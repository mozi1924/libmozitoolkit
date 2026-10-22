//! # LabPBR 1.3 Decoder
//!
//! Decodes raw `_n` / `_s` RGBA8 buffers into the canonical `MtkSurface` channels,
//! packed as `MtkPackedV1`:
//!
//! - `normal_gl`: RGB = OpenGL tangent normal, A = height
//! - `orm`:       R = AO, G = roughness, B = metallic, A = emission mask
//! - `aux`:       R = encoded specular IOR, G = porosity, B = SSS weight, A = transmission
//!
//! All arithmetic is `f32`; the packed output is 8-bit. The formulas are the
//! canonical contract documented in `pbr_channel_spec.md`.

use mtk_texture::RgbaBuffer;

use crate::error::MaterialError;
use crate::pbr::types::{DecodedPbr, PbrMeta};

/// Old decoder's default `_s.R` when a chunk has no specular companion.
/// `1.0 - 0.292893 = 0.707107` perceptual roughness.
const DEFAULT_SMOOTHNESS: f32 = 0.292_893;
const DEFAULT_IOR: f32 = 1.5;
const MAX_IOR: f32 = 8.0;

/// Decode a LabPBR 1.3 `_n` / `_s` pair into canonical packed channels.
///
/// The two inputs may be `None` independently. When both are `None` the result
/// is empty and only carries metadata.
pub fn decode_labpbr13(
    normal: Option<&RgbaBuffer>,
    specular: Option<&RgbaBuffer>,
) -> Result<DecodedPbr, MaterialError> {
    let meta = PbrMeta::labpbr13();

    if normal.is_none() && specular.is_none() {
        return Ok(DecodedPbr {
            normal_gl: None,
            orm: None,
            aux: None,
            meta,
        });
    }

    let (width, height) = normal
        .map(|buffer| (buffer.width, buffer.height))
        .or_else(|| specular.map(|buffer| (buffer.width, buffer.height)))
        .unwrap_or((0, 0));

    if let Some(buffer) = normal {
        ensure_size(buffer, width, height)?;
    }
    if let Some(buffer) = specular {
        ensure_size(buffer, width, height)?;
    }
    if width == 0 || height == 0 {
        return Ok(DecodedPbr {
            normal_gl: None,
            orm: None,
            aux: None,
            meta,
        });
    }

    let mut normal_gl = normal.map(|_| RgbaBuffer::new(width, height));
    let mut orm = Some(RgbaBuffer::new(width, height));
    let mut aux = specular.map(|_| RgbaBuffer::new(width, height));

    // Defaults are written once so a normal-only chunk still gets a sane ORM sheet.
    if let Some(buffer) = orm.as_mut() {
        let default_roughness = encode_unit(1.0 - DEFAULT_SMOOTHNESS);
        for pixel in buffer.pixels.as_chunks_mut::<4>().0 {
            pixel[0] = 255;
            pixel[1] = default_roughness;
            pixel[2] = 0;
            pixel[3] = 0;
        }
    }
    if let Some(buffer) = aux.as_mut() {
        let default_ior = encode_ior(DEFAULT_IOR);
        for pixel in buffer.pixels.as_chunks_mut::<4>().0 {
            pixel[0] = default_ior;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 0;
        }
    }

    let width_usize = width as usize;
    let row_stride = width_usize * 4;

    for y in 0..height as usize {
        let row_start = y * row_stride;
        for x in 0..width_usize {
            let offset = row_start + x * 4;

            if let Some(source) = normal {
                let red = source.pixels[offset];
                let green = source.pixels[offset + 1];
                let blue = source.pixels[offset + 2];
                let alpha = source.pixels[offset + 3];

                let normal_x = 2.0 * (red as f32 / 255.0) - 1.0;
                let normal_y = 1.0 - 2.0 * (green as f32 / 255.0);
                let normal_z = (1.0 - normal_x * normal_x - normal_y * normal_y)
                    .max(0.0)
                    .sqrt();

                if let Some(target) = normal_gl.as_mut() {
                    target.pixels[offset] = red;
                    target.pixels[offset + 1] = 255 - green;
                    target.pixels[offset + 2] = encode_unit(0.5 * normal_z + 0.5);
                    target.pixels[offset + 3] = alpha;
                }
                if let Some(target) = orm.as_mut() {
                    target.pixels[offset] = blue;
                }
            }

            if let Some(source) = specular {
                let red = source.pixels[offset];
                let green = source.pixels[offset + 1];
                let blue = source.pixels[offset + 2];
                let alpha = source.pixels[offset + 3];

                if let Some(target) = orm.as_mut() {
                    target.pixels[offset + 1] = 255 - red;
                    target.pixels[offset + 2] = if green >= 230 { 255 } else { 0 };
                    target.pixels[offset + 3] = emission_byte(alpha);
                }
                if let Some(target) = aux.as_mut() {
                    target.pixels[offset] = encode_ior(ior_from_specular(green));
                    target.pixels[offset + 1] = porosity_byte(blue);
                    target.pixels[offset + 2] = sss_byte(blue);
                    target.pixels[offset + 3] = 0;
                }
            }
        }
    }

    Ok(DecodedPbr {
        normal_gl,
        orm,
        aux,
        meta,
    })
}

/// Decode the 8-bit `aux.R` IOR encoding back to a scalar IOR.
#[inline]
pub fn decode_ior(encoded: u8) -> f32 {
    encoded as f32 / 255.0 * 7.0 + 1.0
}

#[inline]
fn ensure_size(buffer: &RgbaBuffer, width: u32, height: u32) -> Result<(), MaterialError> {
    if buffer.width == width && buffer.height == height {
        Ok(())
    } else {
        Err(MaterialError::PbrSizeMismatch {
            expected_width: width,
            expected_height: height,
            actual_width: buffer.width,
            actual_height: buffer.height,
        })
    }
}

/// LabPBR dielectric F0 / metal threshold -> IOR.
#[inline]
fn ior_from_specular(green: u8) -> f32 {
    if green >= 230 {
        return DEFAULT_IOR;
    }
    let f0 = green.min(229) as f32 / 255.0;
    let root = f0.sqrt();
    let denominator = 1.0 - root;
    if denominator <= 1e-6 {
        return MAX_IOR;
    }
    ((1.0 + root) / denominator).clamp(1.0, MAX_IOR)
}

/// LabPBR `_s.A`: 255 means "no data", 254 means 100%.
#[inline]
fn emission_byte(alpha: u8) -> u8 {
    if alpha == 255 {
        0
    } else {
        encode_unit(alpha.min(254) as f32 / 254.0)
    }
}

/// LabPBR `_s.B` 0..64 is porosity (64 = 100%).
#[inline]
fn porosity_byte(blue: u8) -> u8 {
    if blue <= 64 {
        encode_unit(blue as f32 / 64.0)
    } else {
        0
    }
}

/// LabPBR `_s.B` 65..255 is SSS (65 = 0%, 255 = 100%).
#[inline]
fn sss_byte(blue: u8) -> u8 {
    if blue >= 65 {
        encode_unit((blue - 65) as f32 / 190.0)
    } else {
        0
    }
}

#[inline]
fn encode_unit(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

#[inline]
fn encode_ior(ior: f32) -> u8 {
    encode_unit((ior.clamp(1.0, MAX_IOR) - 1.0) / (MAX_IOR - 1.0))
}
