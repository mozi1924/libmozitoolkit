use std::cmp::Ordering;
use crate::error::TextureError;
use crate::stitcher::region::StitcherRegion;

/// Helper: find smallest power of two greater than or equal to `val`.
#[inline]
pub fn smallest_encompassing_power_of_two(val: u32) -> u32 {
    if val == 0 {
        return 1;
    }
    let mut v = val - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

/// An entry registered into the Stitcher before layout.
#[derive(Debug, Clone)]
pub struct StitcherHolder<T> {
    pub entry: T,
    pub width: u32,
    pub height: u32,
    pub name: String,
}

/// A placed sprite layout slot.
#[derive(Debug, Clone)]
pub struct StitchedSlot<T> {
    pub entry: T,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub chunk_id: u16,
}

/// Result of stitching one or more atlas sheets (chunks).
#[derive(Debug, Clone)]
pub struct StitchedAtlas<T> {
    pub chunks: Vec<StitchedChunk<T>>,
}

#[derive(Debug, Clone)]
pub struct StitchedChunk<T> {
    pub chunk_id: u16,
    pub width: u32,
    pub height: u32,
    pub slots: Vec<StitchedSlot<T>>,
}

/// Vanilla-style binary partitioning 2D Texture Stitcher.
pub struct Stitcher<T> {
    max_width: u32,
    max_height: u32,
    mip_level: u32,
    holders: Vec<StitcherHolder<T>>,
}

impl<T: Clone> Stitcher<T> {
    /// Create a new Stitcher with maximum atlas dimensions and mipmap level.
    pub fn new(max_width: u32, max_height: u32, mip_level: u32) -> Self {
        Self {
            max_width,
            max_height,
            mip_level,
            holders: Vec::new(),
        }
    }

    /// Register a sprite entry with its single-frame width, height, and canonical name.
    pub fn register_sprite(&mut self, entry: T, width: u32, height: u32, name: impl Into<String>) {
        // Enforce mipmap alignment: width and height must be aligned to 1 << mip_level
        let align = 1 << self.mip_level;
        let padded_w = if width % align != 0 {
            ((width / align) + 1) * align
        } else {
            width
        };
        let padded_h = if height % align != 0 {
            ((height / align) + 1) * align
        } else {
            height
        };

        self.holders.push(StitcherHolder {
            entry,
            width: padded_w,
            height: padded_h,
            name: name.into(),
        });
    }

    /// Number of registered entries.
    pub fn len(&self) -> usize {
        self.holders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.holders.is_empty()
    }

    /// Execute the vanilla layout algorithm.
    /// Sorts entries by Height desc, Width desc, Name asc, then packs into binary partitioned regions.
    /// If entries exceed max dimensions, automatically allocates subsequent atlas chunks (pages).
    pub fn stitch(&mut self) -> Result<StitchedAtlas<T>, TextureError> {
        if self.holders.is_empty() {
            return Ok(StitchedAtlas { chunks: Vec::new() });
        }

        // 1. Sort holders strictly following Minecraft vanilla HOLDER_COMPARATOR:
        // Height desc -> Width desc -> Name asc
        self.holders.sort_by(|a, b| {
            let h_ord = b.height.cmp(&a.height);
            if h_ord != Ordering::Equal {
                return h_ord;
            }
            let w_ord = b.width.cmp(&a.width);
            if w_ord != Ordering::Equal {
                return w_ord;
            }
            a.name.cmp(&b.name)
        });

        let mut chunks = Vec::new();
        let mut unplaced = self.holders.clone();
        let mut chunk_id = 0u16;

        while !unplaced.is_empty() {
            let (chunk, remaining) = self.stitch_single_chunk(chunk_id, unplaced)?;
            chunks.push(chunk);
            unplaced = remaining;
            chunk_id += 1;
        }

        Ok(StitchedAtlas { chunks })
    }

    fn stitch_single_chunk(
        &self,
        chunk_id: u16,
        candidates: Vec<StitcherHolder<T>>,
    ) -> Result<(StitchedChunk<T>, Vec<StitcherHolder<T>>), TextureError> {
        let mut storage: Vec<StitcherRegion<StitcherHolder<T>>> = Vec::new();
        let mut storage_x = 0u32;
        let mut storage_y = 0u32;
        let mut unplaced = Vec::new();

        for holder in candidates {
            let w = holder.width;
            let h = holder.height;

            if w > self.max_width || h > self.max_height {
                return Err(TextureError::Stitcher(format!(
                    "Sprite '{}' dimensions ({}x{}) exceed maximum atlas size ({}x{})",
                    holder.name, w, h, self.max_width, self.max_height
                )));
            }

            // 1. Try placing into existing storage regions
            let mut placed = false;
            for region in storage.iter_mut() {
                if region.add(holder.clone(), w, h).is_none() {
                    placed = true;
                    break;
                }
            }

            if placed {
                continue;
            }

            // 2. Try expanding storage canvas
            if self.expand(&mut storage, &mut storage_x, &mut storage_y, &holder) {
                continue;
            }

            // 3. Cannot fit in current chunk -> push to unplaced for next chunk
            unplaced.push(holder);
        }

        // Collect all placed slots
        let mut slots = Vec::new();
        for region in &storage {
            region.walk(&mut |h, x, y, rw, rh| {
                slots.push(StitchedSlot {
                    entry: h.entry.clone(),
                    x,
                    y,
                    width: rw,
                    height: rh,
                    chunk_id,
                });
            });
        }

        let final_w = smallest_encompassing_power_of_two(storage_x);
        let final_h = smallest_encompassing_power_of_two(storage_y);

        Ok((
            StitchedChunk {
                chunk_id,
                width: final_w,
                height: final_h,
                slots,
            },
            unplaced,
        ))
    }

    fn expand(
        &self,
        storage: &mut Vec<StitcherRegion<StitcherHolder<T>>>,
        storage_x: &mut u32,
        storage_y: &mut u32,
        holder: &StitcherHolder<T>,
    ) -> bool {
        let cur_pow_x = smallest_encompassing_power_of_two(*storage_x);
        let cur_pow_y = smallest_encompassing_power_of_two(*storage_y);
        let next_pow_x = smallest_encompassing_power_of_two(*storage_x + holder.width);
        let next_pow_y = smallest_encompassing_power_of_two(*storage_y + holder.height);

        let can_expand_x = next_pow_x <= self.max_width;
        let can_expand_y = next_pow_y <= self.max_height;

        if !can_expand_x && !can_expand_y {
            return false;
        }

        let is_expand_x_diff = can_expand_x && (cur_pow_x != next_pow_x);
        let is_expand_y_diff = can_expand_y && (cur_pow_y != next_pow_y);

        let expand_horizontal = if is_expand_x_diff ^ is_expand_y_diff {
            is_expand_x_diff
        } else if can_expand_x && cur_pow_x <= cur_pow_y {
            true
        } else {
            false
        };

        if expand_horizontal {
            if *storage_y == 0 {
                *storage_y = next_pow_y;
            }
            let mut new_region = StitcherRegion::new(*storage_x, 0, next_pow_x - *storage_x, *storage_y);
            *storage_x = next_pow_x;
            new_region.add(holder.clone(), holder.width, holder.height);
            storage.push(new_region);
        } else {
            if *storage_x == 0 {
                *storage_x = next_pow_x;
            }
            let mut new_region = StitcherRegion::new(0, *storage_y, *storage_x, next_pow_y - *storage_y);
            *storage_y = next_pow_y;
            new_region.add(holder.clone(), holder.width, holder.height);
            storage.push(new_region);
        }

        true
    }
}
