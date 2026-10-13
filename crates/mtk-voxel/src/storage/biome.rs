//! # 3D Quart Biome Storage
//!
//! Provides dense, paletted 3D 4x4x4 quart biome storage for 16x16x16 chunk sections,
//! aligned with Minecraft 1.18+ (Caves & Cliffs) section data format.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// 3D 4x4x4 quart paletted biome storage for a 16x16x16 chunk section.
///
/// Contains 64 total quart cells, each representing a 4x4x4 block region.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SectionBiomes {
    /// Biome resource identifiers (e.g. `["minecraft:taiga", "minecraft:lush_caves"]`).
    pub palette: Vec<String>,
    /// 64 palette indices mapping each 4x4x4 block quart `(qy << 4) | (qz << 2) | qx`.
    /// When `palette.len() <= 1`, this is empty (all quarts implicitly index 0).
    pub indices: Vec<u8>,
    /// Bitmask of quarts explicitly assigned via `set_biome` or `new`.
    #[cfg_attr(feature = "serde", serde(default))]
    explicit_mask: u64,
}

impl SectionBiomes {
    /// Creates a new `SectionBiomes` instance.
    pub fn new(palette: Vec<String>, indices: Vec<u8>) -> Self {
        let explicit_mask = if indices.is_empty() { 0 } else { u64::MAX };
        Self {
            palette,
            indices,
            explicit_mask,
        }
    }

    /// Creates a uniform `SectionBiomes` with a single biome for all 64 quarts.
    pub fn uniform(biome: impl Into<String>) -> Self {
        Self {
            palette: vec![biome.into()],
            indices: Vec::new(),
            explicit_mask: 0,
        }
    }

    /// Returns the biome identifier for local section block coordinate `(lx, ly, lz)`.
    #[inline]
    pub fn get_biome(&self, lx: usize, ly: usize, lz: usize) -> Option<&str> {
        if self.palette.is_empty() {
            return None;
        }
        if self.palette.len() == 1 || self.indices.is_empty() {
            return Some(&self.palette[0]);
        }
        let qx = (lx >> 2) & 3;
        let qy = (ly >> 2) & 3;
        let qz = (lz >> 2) & 3;
        let idx = (qy << 4) | (qz << 2) | qx;
        if let Some(&pal_idx) = self.indices.get(idx) {
            self.palette
                .get(pal_idx as usize)
                .map(|s| s.as_str())
                .or_else(|| self.palette.first().map(|s| s.as_str()))
        } else {
            Some(&self.palette[0])
        }
    }

    /// Sets the biome identifier for local section block coordinate `(lx, ly, lz)`.
    pub fn set_biome(&mut self, lx: usize, ly: usize, lz: usize, biome: &str) {
        let qx = (lx >> 2) & 3;
        let qy = (ly >> 2) & 3;
        let qz = (lz >> 2) & 3;
        let idx = (qy << 4) | (qz << 2) | qx;
        self.explicit_mask |= 1u64 << idx;
        self.set_quart_biome(qx, qy, qz, biome);
        for other_qy in 0..4 {
            let other_idx = (other_qy << 4) | (qz << 2) | qx;
            if (self.explicit_mask & (1u64 << other_idx)) == 0 {
                self.set_quart_biome(qx, other_qy, qz, biome);
            }
        }
    }

    /// Sets the biome identifier for all 4 quarts in local column `(lx, lz)`.
    pub fn set_column_biome(&mut self, lx: usize, lz: usize, biome: &str) {
        let qx = (lx >> 2) & 3;
        let qz = (lz >> 2) & 3;
        for qy in 0..4 {
            let idx = (qy << 4) | (qz << 2) | qx;
            self.explicit_mask |= 1u64 << idx;
            self.set_quart_biome(qx, qy, qz, biome);
        }
    }

    /// Sets the biome identifier for quart coordinate `(qx, qy, qz)` where each is in `0..4`.
    pub fn set_quart_biome(&mut self, qx: usize, qy: usize, qz: usize, biome: &str) {
        let idx = ((qy & 3) << 4) | ((qz & 3) << 2) | (qx & 3);
        if self.indices.is_empty() && self.palette.len() <= 1 {
            self.indices = vec![0; 64];
        }
        let pal_idx = if let Some(pos) = self.palette.iter().position(|p| p == biome) {
            pos as u8
        } else {
            self.palette.push(biome.to_string());
            (self.palette.len() - 1) as u8
        };
        if self.indices.len() == 64 {
            self.indices[idx] = pal_idx;
        }
    }

    /// Returns the biome identifier for quart coordinate `(qx, qy, qz)` where each is in `0..4`.
    #[inline]
    pub fn get_quart_biome(&self, qx: usize, qy: usize, qz: usize) -> Option<&str> {
        if self.palette.is_empty() {
            return None;
        }
        if self.palette.len() == 1 || self.indices.is_empty() {
            return Some(&self.palette[0]);
        }
        let idx = ((qy & 3) << 4) | ((qz & 3) << 2) | (qx & 3);
        if let Some(&pal_idx) = self.indices.get(idx) {
            self.palette
                .get(pal_idx as usize)
                .map(|s| s.as_str())
                .or_else(|| self.palette.first().map(|s| s.as_str()))
        } else {
            Some(&self.palette[0])
        }
    }

    /// Returns true if all 64 quarts share an identical biome.
    #[inline]
    pub fn is_uniform(&self) -> bool {
        if self.palette.len() <= 1 || self.indices.is_empty() {
            return true;
        }
        let first = self.indices[0];
        self.indices.iter().all(|&idx| idx == first)
    }

    /// Returns true if the biomes vary along the Y axis.
    pub fn varies_vertically(&self) -> bool {
        if self.is_uniform() {
            return false;
        }
        for qx in 0..4 {
            for qz in 0..4 {
                let first_b = self.get_quart_biome(qx, 0, qz);
                for qy in 1..4 {
                    if self.get_quart_biome(qx, qy, qz) != first_b {
                        return true;
                    }
                }
            }
        }
        false
    }
}
