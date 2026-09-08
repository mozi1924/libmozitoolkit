//! Centralized physical, geometric, and voxel constants for MoziToolKit.

use crate::geometry::Aabb2d;
use glam::Vec2;

/// Chunk section and voxel domain constants.
pub mod voxel {
    /// Dimension size of a Minecraft chunk section along each axis (X, Y, Z).
    pub const SECTION_SIZE: usize = 16;

    /// Bitmask for coordinates within a 16-wide section (`15` or `0b1111`).
    pub const SECTION_MASK: usize = 15;

    /// Bit-shift count for 16-wide section divisions (`4`).
    pub const SECTION_SHIFT: usize = 4;

    /// 2D area (number of blocks) in a single horizontal slice of a section (`16 * 16 = 256`).
    pub const SECTION_AREA: usize = SECTION_SIZE * SECTION_SIZE;

    /// Total voxel volume within a single 16x16x16 chunk section (`4096`).
    pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE;

    /// Padded array dimension along each axis, including 1-block neighbor boundary on all 6 sides (`18`).
    pub const PADDED_SIZE: usize = 18;

    /// Total padded volume for a 1-block extended neighborhood buffer (`18 * 18 * 18 = 5832`).
    pub const PADDED_VOLUME: usize = PADDED_SIZE * PADDED_SIZE * PADDED_SIZE;

    /// Computes the flat linear index inside a 16x16x16 section storage.
    ///
    /// Standard coordinate layout: `x * 256 + y * 16 + z` (or `(x << 8) | (y << 4) | z`).
    #[inline(always)]
    pub const fn block_index(x: usize, y: usize, z: usize) -> usize {
        (x << 8) | (y << 4) | z
    }

    /// Computes the flat linear index in an 18x18x18 padded neighborhood storage buffer.
    ///
    /// Coordinate layout: `px * 324 + py * 18 + pz`.
    #[inline(always)]
    pub const fn padded_index(px: usize, py: usize, pz: usize) -> usize {
        px * (PADDED_SIZE * PADDED_SIZE) + py * PADDED_SIZE + pz
    }

}

/// Geometry, tolerance, and UV coordinate domain constants.
pub mod geometry {
    use super::*;

    /// Standard floating-point epsilon tolerance for geometric comparisons (`1e-4`).
    pub const EPS: f32 = 1e-4;

    /// Strict tolerance for intersection and clipping (`1e-5`).
    pub const STRICT_EPS: f32 = 1e-5;

    /// Fine numerical tolerance (`1e-6`).
    pub const TOLERANCE: f32 = 1e-6;

    /// Standard texture UV mapping space scale per block face (`16.0`).
    pub const BLOCK_UV_SIZE: f32 = 16.0;

    /// Canonical full-face 2D bounding rectangle in `[0.0..1.0, 0.0..1.0]` normalized space.
    pub const FULL_FACE_RECT: Aabb2d = Aabb2d::UNIT;

    /// Canonical empty/degenerate 2D bounding rectangle.
    pub const EMPTY_FACE_RECT: Aabb2d = Aabb2d::ZERO;

    /// Canonical 2D bounding rectangle in `[0.0..16.0, 0.0..16.0]` block texture space.
    pub const BLOCK_UV_RECT: Aabb2d = Aabb2d {
        min: Vec2::ZERO,
        max: Vec2::new(BLOCK_UV_SIZE, BLOCK_UV_SIZE),
    };

}

/// Fluid physics and rendering constants.
pub mod fluid {
    /// Maximum height of a standard Minecraft still fluid block: `8/9` (~`0.8888889`).
    pub const MAX_FLUID_HEIGHT: f32 = 8.0 / 9.0;

    /// Height multiplier for a falling fluid column: `8/9`.
    pub const FLUID_FALLING_HEIGHT: f32 = 8.0 / 9.0;

    /// Height step per fluid level gradient: `1/9` (~`0.1111111`).
    pub const FLUID_LEVEL_STEP: f32 = 1.0 / 9.0;

    /// Small offset applied to avoid Z-fighting on fluid corners (`0.001`).
    pub const FLUID_CORNER_OFFSET: f32 = 0.001;
}

/// Lighting, ambient occlusion, and brightness constants.
pub mod lighting {
    /// Maximum block / sky light level (`15`).
    pub const MAX_LIGHT_LEVEL: u8 = 15;

    /// Linear brightness multipliers for ambient occlusion discrete levels `0..=3`.
    ///
    /// - Level 0: `0.2`
    /// - Level 1: `0.466`
    /// - Level 2: `0.733`
    /// - Level 3: `1.0`
    pub const AO_LEVEL_MULTIPLIERS: [f32; 4] = [0.2, 0.466, 0.733, 1.0];
}

/// Concurrency, thread pool, and parallel scheduling constants & helpers.
pub mod concurrency {
    /// Safe hardware concurrency detector that works across Native OS and WebAssembly.
    ///
    /// - On native targets with `std`, retrieves `std::thread::available_parallelism()`.
    /// - On `wasm32` or single-threaded targets, returns `1`.
    #[inline]
    pub fn get_safe_hardware_concurrency() -> usize {
        #[cfg(all(feature = "std", not(target_arch = "wasm32")))]
        {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        }
        #[cfg(any(not(feature = "std"), target_arch = "wasm32"))]
        {
            1
        }
    }

    /// Conservative thread count calculator: min(max_cap, max(1, available / 2)).
    /// Leaves headroom for host UI (e.g. Blender) and OS interactivity.
    #[inline]
    pub fn determine_conservative_threads(max_cap: usize) -> usize {
        let available = get_safe_hardware_concurrency();
        (available / 2).clamp(1, max_cap.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_constants() {
        assert_eq!(voxel::SECTION_SIZE, 16);
        assert_eq!(voxel::SECTION_VOLUME, 4096);
        assert_eq!(voxel::PADDED_SIZE, 18);
        assert_eq!(voxel::PADDED_VOLUME, 5832);
        assert_eq!(voxel::block_index(0, 0, 0), 0);
        assert_eq!(voxel::block_index(1, 0, 0), 256);
        assert_eq!(voxel::block_index(0, 1, 0), 16);
        assert_eq!(voxel::block_index(0, 0, 1), 1);
        assert_eq!(voxel::block_index(15, 15, 15), 4095);
    }

    #[test]
    fn test_geometry_constants() {
        assert_eq!(geometry::FULL_FACE_RECT.min, Vec2::ZERO);
        assert_eq!(geometry::FULL_FACE_RECT.max, Vec2::ONE);
        assert_eq!(geometry::EMPTY_FACE_RECT.min, Vec2::ZERO);
        assert_eq!(geometry::EMPTY_FACE_RECT.max, Vec2::ZERO);
        assert_eq!(geometry::BLOCK_UV_RECT.max, Vec2::new(16.0, 16.0));
    }

    #[test]
    fn test_fluid_and_lighting_constants() {
        assert!((fluid::MAX_FLUID_HEIGHT - 8.0 / 9.0).abs() < 1e-6);
        assert_eq!(lighting::MAX_LIGHT_LEVEL, 15);
        assert_eq!(lighting::AO_LEVEL_MULTIPLIERS.len(), 4);
    }

    #[test]
    fn test_concurrency_helpers() {
        let conc = concurrency::get_safe_hardware_concurrency();
        assert!(conc >= 1);
        let cons = concurrency::determine_conservative_threads(8);
        assert!((1..=8).contains(&cons));
    }
}

