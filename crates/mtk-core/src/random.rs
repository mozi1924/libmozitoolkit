//! # Minecraft Procedural Random & Coordinate Offset Core
//!
//! Provides 1:1 binary-compatible implementations of Minecraft Java Edition's:
//! - Coordinate-based random seed calculation (`net.minecraft.util.Mth.getSeed`)
//! - 48-bit Linear Congruential Generator (`java.util.Random` / `SingleThreadedRandomSource`)
//! - Plant and foliage coordinate jitter offsets (`BlockBehaviour$Properties.offsetType`)

use glam::Vec3;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// 1:1 parity with Minecraft Java Edition `net.minecraft.util.Mth.getSeed(int x, int y, int z)`.
///
/// Computes a deterministic 64-bit pseudo-random seed from integer world coordinates.
#[inline]
pub fn mc_coordinate_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut l =
        ((x as i64).wrapping_mul(3129871)) ^ ((z as i64).wrapping_mul(116129781)) ^ (y as i64);
    l = l
        .wrapping_mul(l)
        .wrapping_mul(42317861)
        .wrapping_add(l.wrapping_mul(11));
    l >> 16
}

/// 1:1 parity with Java's standard 48-bit Linear Congruential Generator (`java.util.Random`).
///
/// Used by Minecraft's `WeightedVariants` and `ModelBlockRenderer` for deterministic variant selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    /// Multiplier: 25214903917 (0x5DEECE66D).
    const MULTIPLIER: u64 = 0x5DEECE66Du64;
    /// Addend: 11.
    const ADDEND: u64 = 11u64;
    /// 48-bit mask ((1 << 48) - 1).
    const MASK: u64 = (1u64 << 48) - 1;

    /// Creates a new `JavaRandom` seeded with the given signed 64-bit integer.
    #[inline]
    pub fn new(seed: i64) -> Self {
        let mut r = Self { seed: 0 };
        r.set_seed(seed);
        r
    }

    /// Sets the seed using Java's scrambling algorithm: `(seed ^ MULTIPLIER) & MASK`.
    #[inline]
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ Self::MULTIPLIER) & Self::MASK;
    }

    /// Advances the LCG and returns the upper `bits` (1 to 32) of the 48-bit state.
    #[inline]
    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND))
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Generates a pseudo-random integer uniformly distributed in `[0, bound)`.
    #[inline]
    pub fn next_int(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        // Power-of-two fast path (identical to java.util.Random)
        if (bound & (bound - 1)) == 0 {
            return (((bound as u64).wrapping_mul(self.next(31) as u64)) >> 31) as u32;
        }

        // Standard rejection sampling
        let mut bits = self.next(31);
        let mut val = (bits as u32) % bound;
        while (bits as u32).wrapping_sub(val).wrapping_add(bound - 1) & 0x80000000 != 0 {
            bits = self.next(31);
            val = (bits as u32) % bound;
        }
        val
    }

    /// Generates a pseudo-random float in `[0.0, 1.0)`.
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / ((1u32 << 24) as f32)
    }
}

/// Offset category corresponding to `BlockBehaviour$OffsetType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OffsetType {
    /// Center-aligned on block grid, no jitter.
    #[default]
    None,
    /// XZ horizontal jitter only (e.g. flowers, tall grass, roots).
    XZ,
    /// XYZ 3D jitter with horizontal offset and downward sinking (e.g. short grass, fern, mangrove roots).
    XYZ,
}

/// 1:1 parity with Minecraft's `BlockState.getOffset(world, pos)`.
///
/// Computes a continuous position offset vector for plants and foliage.
/// Note that Minecraft uses `Mth.getSeed(x, 0, z)` with `y = 0` so vertical plant columns
/// (or multi-block plants) share the exact same horizontal displacement.
#[inline]
pub fn get_block_offset(offset_type: OffsetType, x: i32, _y: i32, z: i32) -> Vec3 {
    match offset_type {
        OffsetType::None => Vec3::ZERO,
        OffsetType::XZ => {
            let seed = mc_coordinate_seed(x, 0, z);
            let dx = (((seed & 15) as f32 / 15.0 - 0.5) * 0.5).clamp(-0.25, 0.25);
            let dz = ((((seed >> 8) & 15) as f32 / 15.0 - 0.5) * 0.5).clamp(-0.25, 0.25);
            Vec3::new(dx, 0.0, dz)
        }
        OffsetType::XYZ => {
            let seed = mc_coordinate_seed(x, 0, z);
            let dx = (((seed & 15) as f32 / 15.0 - 0.5) * 0.5).clamp(-0.25, 0.25);
            let dy = (((seed >> 4) & 15) as f32 / 15.0 - 1.0) * 0.2;
            let dz = ((((seed >> 8) & 15) as f32 / 15.0 - 0.5) * 0.5).clamp(-0.25, 0.25);
            Vec3::new(dx, dy, dz)
        }
    }
}

/// Determines the standard vanilla `OffsetType` for a canonical block identifier.
///
/// Identifiers can be raw block IDs like `"minecraft:short_grass"` or full BlockState strings.
pub fn determine_block_offset_type(block_id_or_state: &str) -> OffsetType {
    let name = block_id_or_state
        .strip_prefix("minecraft:")
        .unwrap_or(block_id_or_state);
    let base = name.split('[').next().unwrap_or(name);

    match base {
        // XYZ: Short grass, fern, dry grass, small dripleaf, pointed dripstone, mangrove roots
        "short_grass" | "grass" | "fern" | "short_dry_grass" | "small_dripleaf"
        | "pointed_dripstone" | "mangrove_roots" => OffsetType::XYZ,

        // XZ: Wildflowers, tulips, poppy, dandelion, tall flowers, roots, saplings
        "dandelion" | "poppy" | "blue_orchid" | "allium" | "azure_bluet" | "red_tulip"
        | "orange_tulip" | "white_tulip" | "pink_tulip" | "oxeye_daisy" | "cornflower"
        | "wither_rose" | "lily_of_the_valley" | "sunflower" | "lilac" | "rose_bush" | "peony"
        | "tall_grass" | "large_fern" | "pitcher_plant" | "bamboo_sapling" | "warped_roots"
        | "nether_sprouts" | "crimson_roots" | "hanging_roots" | "open_eyeblossom"
        | "closed_eyeblossom" | "torchflower" | "mangrove_propagule" | "dead_bush" => {
            OffsetType::XZ
        }

        _ => OffsetType::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mc_coordinate_seed_parity() {
        assert_eq!(mc_coordinate_seed(0, 0, 0), 0);
        assert_eq!(mc_coordinate_seed(10, 64, -20), -62212282568415);
        assert_eq!(mc_coordinate_seed(15, 0, -25), -97009344846680);
    }

    #[test]
    fn test_java_random_parity() {
        let seed = mc_coordinate_seed(10, 64, -20);
        let mut rng = JavaRandom::new(seed);
        assert_eq!(rng.next_int(4), 0);
        assert_eq!(rng.next_int(4), 0);
        assert_eq!(rng.next_int(4), 1);
    }

    #[test]
    fn test_plant_offset_parity() {
        let offset_xyz = get_block_offset(OffsetType::XYZ, 15, 0, -25);
        let expected_dx = 0.01666668;
        let expected_dy = -0.06666666;
        let expected_dz = -0.11666666;

        assert!((offset_xyz.x - expected_dx).abs() < 1e-6);
        assert!((offset_xyz.y - expected_dy).abs() < 1e-6);
        assert!((offset_xyz.z - expected_dz).abs() < 1e-6);

        // Verify Y-independence (plant stems at different heights keep identical horizontal offset)
        let offset_at_height = get_block_offset(OffsetType::XYZ, 15, 80, -25);
        assert_eq!(offset_xyz.x, offset_at_height.x);
        assert_eq!(offset_xyz.z, offset_at_height.z);

        // Verify bounds
        for x in -50..50 {
            for z in -50..50 {
                let off_xz = get_block_offset(OffsetType::XZ, x, 0, z);
                assert!(off_xz.x >= -0.25 && off_xz.x <= 0.25);
                assert_eq!(off_xz.y, 0.0);
                assert!(off_xz.z >= -0.25 && off_xz.z <= 0.25);

                let off_xyz = get_block_offset(OffsetType::XYZ, x, 0, z);
                assert!(off_xyz.x >= -0.25 && off_xyz.x <= 0.25);
                assert!(off_xyz.y >= -0.2000001 && off_xyz.y <= 0.0000001);
                assert!(off_xyz.z >= -0.25 && off_xyz.z <= 0.25);
            }
        }
    }

    #[test]
    fn test_determine_block_offset_type() {
        assert_eq!(
            determine_block_offset_type("minecraft:short_grass"),
            OffsetType::XYZ
        );
        assert_eq!(
            determine_block_offset_type("minecraft:fern"),
            OffsetType::XYZ
        );
        assert_eq!(
            determine_block_offset_type("minecraft:poppy"),
            OffsetType::XZ
        );
        assert_eq!(
            determine_block_offset_type("minecraft:dandelion[half=lower]"),
            OffsetType::XZ
        );
        assert_eq!(
            determine_block_offset_type("minecraft:stone"),
            OffsetType::None
        );
        assert_eq!(
            determine_block_offset_type("minecraft:dirt"),
            OffsetType::None
        );
    }
}
