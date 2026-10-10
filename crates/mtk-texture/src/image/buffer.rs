use crate::error::TextureError;
use image::{ImageBuffer, Rgba};
use std::io::Cursor;

/// High-performance 2D RGBA pixel buffer for atlas generation and blitting.
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA 4 bytes per pixel, row-major
}

impl RgbaBuffer {
    /// Create a new blank buffer initialized to transparent black (0, 0, 0, 0).
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            width,
            height,
            pixels: vec![0; size],
        }
    }

    /// Create a new buffer filled with a solid RGBA color.
    pub fn solid(width: u32, height: u32, r: u8, g: u8, b: u8, a: u8) -> Self {
        let mut buf = Self::new(width, height);
        for chunk in buf.pixels.as_chunks_mut::<4>().0 {
            *chunk = [r, g, b, a];
        }
        buf
    }

    /// Create from decoded ImageBuffer<Rgba<u8>, Vec<u8>>.
    pub fn from_image_buffer(img: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        let (width, height) = img.dimensions();
        Self {
            width,
            height,
            pixels: img.into_raw(),
        }
    }

    /// Decode raw PNG bytes into an RgbaBuffer.
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, TextureError> {
        let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?;
        let rgba = img.to_rgba8();
        Ok(Self::from_image_buffer(rgba))
    }

    /// Encode buffer into PNG byte vector.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, TextureError> {
        let img_buf =
            ImageBuffer::<Rgba<u8>, _>::from_raw(self.width, self.height, self.pixels.clone())
                .ok_or_else(|| {
                    TextureError::Baking("Failed to create ImageBuffer from raw pixels".to_string())
                })?;
        let mut cursor = Cursor::new(Vec::new());
        img_buf.write_to(&mut cursor, image::ImageFormat::Png)?;
        Ok(cursor.into_inner())
    }

    #[inline]
    pub fn pixel_index(&self, x: u32, y: u32) -> usize {
        ((y * self.width + x) * 4) as usize
    }

    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0, 0];
        }
        let idx = self.pixel_index(x, y);
        [
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ]
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.pixel_index(x, y);
        self.pixels[idx] = color[0];
        self.pixels[idx + 1] = color[1];
        self.pixels[idx + 2] = color[2];
        self.pixels[idx + 3] = color[3];
    }

    /// Copy a rectangular region from `src` directly onto `self` at destination (dst_x, dst_y).
    pub fn blit(
        &mut self,
        src: &RgbaBuffer,
        src_x: u32,
        src_y: u32,
        dst_x: u32,
        dst_y: u32,
        width: u32,
        height: u32,
    ) {
        let copy_w = width
            .min(src.width.saturating_sub(src_x))
            .min(self.width.saturating_sub(dst_x));
        let copy_h = height
            .min(src.height.saturating_sub(src_y))
            .min(self.height.saturating_sub(dst_y));

        if copy_w == 0 || copy_h == 0 {
            return;
        }

        for row in 0..copy_h {
            let src_start = src.pixel_index(src_x, src_y + row);
            let src_end = src_start + (copy_w * 4) as usize;
            let dst_start = self.pixel_index(dst_x, dst_y + row);
            let dst_end = dst_start + (copy_w * 4) as usize;

            self.pixels[dst_start..dst_end].copy_from_slice(&src.pixels[src_start..src_end]);
        }
    }

    /// Extract a sub-rectangle as a new RgbaBuffer.
    pub fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        let mut sub = Self::new(width, height);
        sub.blit(self, x, y, 0, 0, width, height);
        sub
    }

    /// Tile this buffer vertically until it reaches `target_height`.
    /// Used to align 1-frame or fewer-frame PBR companion textures (normal/specular)
    /// with multi-frame animated albedo textures.
    pub fn tile_vertical(&self, target_height: u32) -> Self {
        if self.height == 0 || self.width == 0 || self.height == target_height {
            return self.clone();
        }

        let mut tiled = Self::new(self.width, target_height);
        let mut y = 0u32;
        while y < target_height {
            let chunk_h = (target_height - y).min(self.height);
            tiled.blit(self, 0, 0, 0, y, self.width, chunk_h);
            y += self.height;
        }
        tiled
    }

    /// Resize buffer to exact (new_width, new_height) using nearest neighbor interpolation.
    /// Preserves crisp pixel boundaries and avoids PBR channel interpolation artifacts.
    pub fn resize_nearest(&self, new_width: u32, new_height: u32) -> Self {
        if self.width == new_width && self.height == new_height {
            return self.clone();
        }
        if new_width == 0 || new_height == 0 || self.width == 0 || self.height == 0 {
            return Self::new(new_width, new_height);
        }

        let mut output = Self::new(new_width, new_height);
        for dy in 0..new_height {
            let sy = (dy as u64 * self.height as u64 / new_height as u64) as u32;
            for dx in 0..new_width {
                let sx = (dx as u64 * self.width as u64 / new_width as u64) as u32;
                let color = self.get_pixel(sx, sy);
                output.set_pixel(dx, dy, color);
            }
        }
        output
    }

    /// Align a PBR companion buffer (`_n` or `_s`) with its corresponding Albedo texture metrics.
    ///
    /// - If companion has matching total dimensions, it is returned as-is.
    /// - If companion is a single-frame texture (or has fewer frames than albedo):
    ///   1. Its single frame is resized to match `(albedo_fw, albedo_fh)`.
    ///   2. If albedo has multiple frames (`albedo_fc > 1`), it is vertically tiled to match `albedo_fh * albedo_fc`.
    /// - If companion is already a multi-frame strip matching `albedo_fc`, it is resized to `(albedo_fw, albedo_fh * albedo_fc)`.
    pub fn align_companion_to_albedo(
        &self,
        albedo_fw: u32,
        albedo_fh: u32,
        albedo_fc: u32,
    ) -> Self {
        let target_total_height = albedo_fh.saturating_mul(albedo_fc.max(1));
        if self.width == albedo_fw && self.height == target_total_height {
            return self.clone();
        }

        if self.width == 0 || self.height == 0 || albedo_fw == 0 || albedo_fh == 0 {
            return self.clone();
        }

        let is_multi_frame_matching =
            albedo_fc > 1 && self.height > self.width && (self.height / self.width) == albedo_fc;

        if is_multi_frame_matching {
            self.resize_nearest(albedo_fw, target_total_height)
        } else {
            let single_frame = if self.height > self.width && albedo_fc > 1 {
                self.crop(0, 0, self.width, self.width)
            } else {
                self.clone()
            };

            let resized_single = single_frame.resize_nearest(albedo_fw, albedo_fh);
            if albedo_fc > 1 {
                resized_single.tile_vertical(target_total_height)
            } else {
                resized_single
            }
        }
    }

    /// In-place vertically flip (V-axis / Y-axis inversion) the pixel rows.
    pub fn flip_v(&mut self) {
        if self.height <= 1 || self.width == 0 {
            return;
        }
        let row_bytes = (self.width * 4) as usize;
        let half = (self.height / 2) as usize;
        let h = self.height as usize;
        for y in 0..half {
            let top_idx = y * row_bytes;
            let bot_idx = (h - 1 - y) * row_bytes;
            let (first, second) = self.pixels.split_at_mut(bot_idx);
            first[top_idx..top_idx + row_bytes].swap_with_slice(&mut second[..row_bytes]);
        }
    }

    /// Returns a vertically flipped copy of the buffer.
    pub fn to_flipped_v(&self) -> Self {
        let mut cloned = self.clone();
        cloned.flip_v();
        cloned
    }

    /// Converts the RGBA8 buffer to a normalized [0.0, 1.0] float32 buffer (`width * height * 4`).
    /// If `flip_v` is true, rows are streamed bottom-to-top (Blender native Image.pixels coordinate system).
    pub fn to_f32_buffer(&self, flip_v: bool) -> Vec<f32> {
        let total_floats = (self.width * self.height * 4) as usize;
        let mut f32_buf = Vec::with_capacity(total_floats);
        let row_bytes = (self.width * 4) as usize;
        let h = self.height as usize;
        let scale = 1.0f32 / 255.0f32;

        if flip_v {
            for y in (0..h).rev() {
                let row_start = y * row_bytes;
                let row_slice = &self.pixels[row_start..row_start + row_bytes];
                for &b in row_slice {
                    f32_buf.push(b as f32 * scale);
                }
            }
        } else {
            for &b in &self.pixels {
                f32_buf.push(b as f32 * scale);
            }
        }
        f32_buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_buffer_flip_v() {
        let mut buf = RgbaBuffer::new(2, 2);
        // Row 0: Red, Green
        buf.set_pixel(0, 0, [255, 0, 0, 255]);
        buf.set_pixel(1, 0, [0, 255, 0, 255]);
        // Row 1: Blue, White
        buf.set_pixel(0, 1, [0, 0, 255, 255]);
        buf.set_pixel(1, 1, [255, 255, 255, 255]);

        buf.flip_v();

        // After flip, Row 0 should be old Row 1 (Blue, White)
        assert_eq!(buf.get_pixel(0, 0), [0, 0, 255, 255]);
        assert_eq!(buf.get_pixel(1, 0), [255, 255, 255, 255]);
        // Row 1 should be old Row 0 (Red, Green)
        assert_eq!(buf.get_pixel(0, 1), [255, 0, 0, 255]);
        assert_eq!(buf.get_pixel(1, 1), [0, 255, 0, 255]);
    }

    #[test]
    fn test_rgba_buffer_to_f32_buffer() {
        let mut buf = RgbaBuffer::new(2, 2);
        // Row 0: Red, Green
        buf.set_pixel(0, 0, [255, 0, 0, 255]);
        buf.set_pixel(1, 0, [0, 255, 0, 255]);
        // Row 1: Blue, White
        buf.set_pixel(0, 1, [0, 0, 255, 255]);
        buf.set_pixel(1, 1, [255, 255, 255, 255]);

        let f32_flipped = buf.to_f32_buffer(true);
        assert_eq!(f32_flipped.len(), 16);
        // Row 0 (bottom row in Blender) should be Blue [0, 0, 1, 1]
        assert!((f32_flipped[0] - 0.0).abs() < 1e-4);
        assert!((f32_flipped[1] - 0.0).abs() < 1e-4);
        assert!((f32_flipped[2] - 1.0).abs() < 1e-4);
        assert!((f32_flipped[3] - 1.0).abs() < 1e-4);

        // Row 1 (top row in Blender) should be Red [1, 0, 0, 1]
        assert!((f32_flipped[8] - 1.0).abs() < 1e-4);
        assert!((f32_flipped[9] - 0.0).abs() < 1e-4);
        assert!((f32_flipped[10] - 0.0).abs() < 1e-4);
        assert!((f32_flipped[11] - 1.0).abs() < 1e-4);
    }
}
