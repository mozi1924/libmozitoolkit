use std::io::Cursor;
use image::{ImageBuffer, Rgba};
use crate::error::TextureError;

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
        for chunk in buf.pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
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
        let img_buf = ImageBuffer::<Rgba<u8>, _>::from_raw(self.width, self.height, self.pixels.clone())
            .ok_or_else(|| TextureError::Baking("Failed to create ImageBuffer from raw pixels".to_string()))?;
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
        [self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2], self.pixels[idx + 3]]
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
        let copy_w = width.min(src.width.saturating_sub(src_x)).min(self.width.saturating_sub(dst_x));
        let copy_h = height.min(src.height.saturating_sub(src_y)).min(self.height.saturating_sub(dst_y));

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
}
