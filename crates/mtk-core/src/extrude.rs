//! # Auto Extrude UV Repair & Random Noise Extrude Generator
//!
//! Provides geometric UV reconstruction for newly extruded side faces, collapsed UV detection,
//! Atlas Safe Padding Clamping, and 3D Perlin / Cellular noise generators for random terrain extrusion.

use alloc::vec::Vec;
use core::f32::consts::PI;

use crate::geometry::Aabb2d;

/// UV repair mode for extruded side polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeUvMode {
    /// Smart projection mode: evaluates dot product of extrude vector and top face normal.
    /// Extrusion outward (>= -1e-6) uses INWARD sampling; indentation (< -1e-6) uses OUTWARD sampling.
    #[default]
    Smart,
    /// Inward sampling: samples from top face perimeter into the interior (by 0.1 pixel step).
    Inward,
    /// Outward sampling: extends UV coordinates from the adjacent base polygon.
    Outward,
}

/// Noise algorithm for random extrusion height generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtrudeNoiseType {
    /// Uniform pseudo-random distribution.
    #[default]
    UniformRandom,
    /// Continuous 3D Perlin gradient noise for smooth undulating terrain.
    Perlin,
    /// Voronoi / Cellular grid noise for flat faceted stone / brick steps.
    Cellular,
}

/// Computes reconstructed 4 UV corner coordinates for an extruded side quad.
///
/// Order of returned UVs matches `[uv_base_a, uv_base_b, uv_top_b, uv_top_a]`.
pub fn repair_extruded_side_uv(
    uv_base_a: [f32; 2],
    uv_base_b: [f32; 2],
    top_normal: [f32; 3],
    extrude_vec: [f32; 3],
    mode: ExtrudeUvMode,
    step_u: f32,
    step_v: f32,
    top_uv_bounds: Aabb2d,
    adjacent_uv_strip: Option<[[f32; 2]; 4]>,
) -> [[f32; 2]; 4] {
    let resolved_mode = match mode {
        ExtrudeUvMode::Smart => {
            let dot = extrude_vec[0] * top_normal[0]
                + extrude_vec[1] * top_normal[1]
                + extrude_vec[2] * top_normal[2];
            if dot >= -1e-6 {
                ExtrudeUvMode::Inward
            } else {
                ExtrudeUvMode::Outward
            }
        }
        other => other,
    };

    if resolved_mode == ExtrudeUvMode::Outward {
        if let Some(strip) = adjacent_uv_strip {
            return strip;
        }
    }

    // Inward mode (sampling narrow inward band from top face perimeter)
    let edge_du = uv_base_b[0] - uv_base_a[0];
    let edge_dv = uv_base_b[1] - uv_base_a[1];

    // Outward 2D normal perpendicular to base edge
    let norm_len = (edge_du * edge_du + edge_dv * edge_dv).sqrt();
    let (out_u, out_v) = if norm_len > 1e-6 {
        (edge_dv / norm_len, -edge_du / norm_len)
    } else {
        (0.0, 1.0)
    };

    let dir_multiplier = match resolved_mode {
        ExtrudeUvMode::Inward => -1.0,
        _ => 1.0,
    };

    let offset_u = out_u * dir_multiplier * (step_u * 0.1);
    let offset_v = out_v * dir_multiplier * (step_v * 0.1);

    let mut base_a = uv_base_a;
    let mut base_b = uv_base_b;
    let mut top_b = [uv_base_b[0] + offset_u, uv_base_b[1] + offset_v];
    let mut top_a = [uv_base_a[0] + offset_u, uv_base_a[1] + offset_v];

    // Atlas Safe Padding Clamping
    let pad_u = (step_u * 0.05).min((top_uv_bounds.max.x - top_uv_bounds.min.x).abs() * 0.1);
    let pad_v = (step_v * 0.05).min((top_uv_bounds.max.y - top_uv_bounds.min.y).abs() * 0.1);

    let min_safe_u = top_uv_bounds.min.x + pad_u;
    let max_safe_u = top_uv_bounds.max.x - pad_u;
    let min_safe_v = top_uv_bounds.min.y + pad_v;
    let max_safe_v = top_uv_bounds.max.y - pad_v;

    if max_safe_u >= min_safe_u {
        base_a[0] = base_a[0].clamp(min_safe_u, max_safe_u);
        base_b[0] = base_b[0].clamp(min_safe_u, max_safe_u);
        top_a[0] = top_a[0].clamp(min_safe_u, max_safe_u);
        top_b[0] = top_b[0].clamp(min_safe_u, max_safe_u);
    }

    if max_safe_v >= min_safe_v {
        base_a[1] = base_a[1].clamp(min_safe_v, max_safe_v);
        base_b[1] = base_b[1].clamp(min_safe_v, max_safe_v);
        top_a[1] = top_a[1].clamp(min_safe_v, max_safe_v);
        top_b[1] = top_b[1].clamp(min_safe_v, max_safe_v);
    }

    [base_a, base_b, top_b, top_a]
}

// =========================================================================
// Deterministic 3D Noise Functions (No heavy external dependencies)
// =========================================================================

/// Simple fast hash function for 3D coordinates.
#[inline]
fn hash_3d(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut n = (x.wrapping_mul(374761393)
        ^ y.wrapping_mul(668265263)
        ^ z.wrapping_mul(951214043)
        ^ (seed as i32).wrapping_mul(374761393)) as u32;
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n ^ (n >> 16)) as f32 / 4294967295.0
}

/// 3D Perlin Gradient Noise in range [-1.0, 1.0].
pub fn perlin_noise_3d(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let z0 = z.floor() as i32;

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let fz = z - z0 as f32;

    // Smoothstep curve (6t^5 - 15t^4 + 10t^3)
    let u = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let v = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);
    let w = fz * fz * fz * (fz * (fz * 6.0 - 15.0) + 10.0);

    let mut result = 0.0;
    for dx in 0..=1 {
        for dy in 0..=1 {
            for dz in 0..=1 {
                let h = hash_3d(x0 + dx, y0 + dy, z0 + dz, seed) * 2.0 * PI;
                let h2 = hash_3d(x0 + dx, y0 + dy, z0 + dz, seed.wrapping_add(101)) * PI;

                let gx = h.cos() * h2.sin();
                let gy = h.sin() * h2.sin();
                let gz = h2.cos();

                let dot = gx * (fx - dx as f32) + gy * (fy - dy as f32) + gz * (fz - dz as f32);

                let wx = if dx == 0 { 1.0 - u } else { u };
                let wy = if dy == 0 { 1.0 - v } else { v };
                let wz = if dz == 0 { 1.0 - w } else { w };

                result += dot * wx * wy * wz;
            }
        }
    }

    result.clamp(-1.0, 1.0)
}

/// 3D Cellular / Voronoi F1 Noise in range [0.0, 1.0].
pub fn cellular_noise_3d(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let z0 = z.floor() as i32;

    let mut min_dist_sq = f32::MAX;

    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let cx = x0 + dx;
                let cy = y0 + dy;
                let cz = z0 + dz;

                let px = cx as f32 + hash_3d(cx, cy, cz, seed);
                let py = cy as f32 + hash_3d(cx, cy, cz, seed.wrapping_add(53));
                let pz = cz as f32 + hash_3d(cx, cy, cz, seed.wrapping_add(97));

                let dist_sq = (px - x) * (px - x) + (py - y) * (py - y) + (pz - z) * (pz - z);
                if dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                }
            }
        }
    }

    min_dist_sq.sqrt().clamp(0.0, 1.0)
}

/// Generates a list of random extrusion height values for face center positions.
pub fn generate_extrude_heights(
    centers: &[[f32; 3]],
    noise_type: ExtrudeNoiseType,
    min_height: f32,
    max_height: f32,
    noise_scale: f32,
    seed: u32,
    discrete_steps: Option<u32>,
) -> Vec<f32> {
    let scale = if noise_scale <= 0.0 { 1.0 } else { noise_scale };
    let height_range = max_height - min_height;

    centers
        .iter()
        .enumerate()
        .map(|(idx, &[x, y, z])| {
            let t = match noise_type {
                ExtrudeNoiseType::UniformRandom => hash_3d(idx as i32, 0, 0, seed),
                ExtrudeNoiseType::Perlin => {
                    (perlin_noise_3d(x * scale, y * scale, z * scale, seed) + 1.0) * 0.5
                }
                ExtrudeNoiseType::Cellular => cellular_noise_3d(x * scale, y * scale, z * scale, seed),
            };

            let mut h = min_height + t * height_range;
            if let Some(steps) = discrete_steps {
                if steps > 1 {
                    let step_val = height_range / (steps - 1) as f32;
                    let step_idx = ((h - min_height) / step_val).round();
                    h = min_height + step_idx * step_val;
                }
            }
            h
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin_and_cellular_noise_bounds() {
        for i in 0..10 {
            let val = perlin_noise_3d(i as f32 * 0.3, i as f32 * 0.5, 0.0, 42);
            assert!(val >= -1.0 && val <= 1.0);

            let cell = cellular_noise_3d(i as f32 * 0.3, i as f32 * 0.5, 0.0, 42);
            assert!(cell >= 0.0 && cell <= 1.0);
        }
    }

    #[test]
    fn test_generate_extrude_heights_discrete() {
        let centers = vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]];
        let heights = generate_extrude_heights(
            &centers,
            ExtrudeNoiseType::UniformRandom,
            0.0,
            1.0,
            1.0,
            123,
            Some(3), // 3 steps: 0.0, 0.5, 1.0
        );
        for h in heights {
            assert!(h == 0.0 || (h - 0.5).abs() < 1e-4 || (h - 1.0).abs() < 1e-4);
        }
    }
}
