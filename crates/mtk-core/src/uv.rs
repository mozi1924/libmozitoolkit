//! High-performance UV geometric algorithms, Shoelace area calculation,
//! bounds, collapse detection, orthogonal angle checking, rotation detection,
//! straightening, scaling, and Atlas tiling affine mappings.

use alloc::vec::Vec;
use core::f32::consts::{FRAC_PI_2, PI};
use glam::{Vec2, Vec3};

use crate::geometry::Aabb2d;

/// Calculate 2D signed area of a polygon loop in UV space ([f32; 2]) using the Shoelace formula.
#[inline]
pub fn calculate_uv_area_2d(uvs: &[[f32; 2]]) -> f32 {
    let n = uvs.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0f32;
    for i in 0..n {
        let p1 = uvs[i];
        let p2 = uvs[(i + 1) % n];
        area += p1[0] * p2[1] - p2[0] * p1[1];
    }
    0.5 * area.abs()
}

/// Calculate 2D signed area of a polygon in UV space using the Shoelace formula.
#[inline]
pub fn calculate_uv_area(uvs: &[Vec2]) -> f32 {
    let n = uvs.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0f32;
    for i in 0..n {
        let uv1 = uvs[i];
        let uv2 = uvs[(i + 1) % n];
        area += uv1.x * uv2.y - uv2.x * uv1.y;
    }
    0.5 * area.abs()
}

/// Calculate 2D axis-aligned bounding box (AABB) of UV loop coordinates.
#[inline]
pub fn get_uv_bounds(uvs: &[Vec2]) -> Aabb2d {
    if uvs.is_empty() {
        return Aabb2d::ZERO;
    }
    let mut min_u = uvs[0].x;
    let mut max_u = uvs[0].x;
    let mut min_v = uvs[0].y;
    let mut max_v = uvs[0].y;

    for uv in &uvs[1..] {
        min_u = min_u.min(uv.x);
        max_u = max_u.max(uv.x);
        min_v = min_v.min(uv.y);
        max_v = max_v.max(uv.y);
    }

    Aabb2d::new(Vec2::new(min_u, min_v), Vec2::new(max_u, max_v))
}

/// Calculate geometric center vector of UV coordinates.
#[inline]
pub fn get_uv_center(uvs: &[Vec2]) -> Vec2 {
    if uvs.is_empty() {
        return Vec2::ZERO;
    }
    let mut sum = Vec2::ZERO;
    for uv in uvs {
        sum += *uv;
    }
    sum / (uvs.len() as f32)
}

/// Check if face UVs ([f32; 2]) are collapsed to a point, line segment, or near zero 2D area.
#[inline]
pub fn is_uv_collapsed_2d(uvs: &[[f32; 2]], pixel_step: Option<[f32; 2]>) -> bool {
    if uvs.len() < 3 {
        return true;
    }
    let (area_thresh, dist_thresh) = if let Some([su, sv]) = pixel_step {
        (su * sv * 0.01, su.min(sv) * 0.02)
    } else {
        (1e-6, 1e-4)
    };

    if calculate_uv_area_2d(uvs) < area_thresh {
        return true;
    }

    let mut max_dist_sq = 0.0f32;
    for i in 0..uvs.len() {
        for j in (i + 1)..uvs.len() {
            let du = uvs[i][0] - uvs[j][0];
            let dv = uvs[i][1] - uvs[j][1];
            let d2 = du * du + dv * dv;
            if d2 > max_dist_sq {
                max_dist_sq = d2;
            }
        }
    }

    max_dist_sq < (dist_thresh * dist_thresh)
}

/// Check if face UVs are collapsed to a point, line segment, or near zero 2D area.
///
/// Adapts dynamically to the face's UV pixel resolution if `pixel_step` is provided.
#[inline]
pub fn is_uv_collapsed(
    uvs: &[Vec2],
    area_threshold: Option<f32>,
    dist_threshold: Option<f32>,
    pixel_step: Option<(f32, f32)>,
) -> bool {
    if uvs.len() < 3 {
        return true;
    }

    let (calc_area_threshold, calc_dist_threshold) = if let Some((step_u, step_v)) = pixel_step {
        (
            area_threshold.unwrap_or(step_u * step_v * 0.05),
            dist_threshold.unwrap_or(step_u.min(step_v) * 0.1),
        )
    } else {
        (
            area_threshold.unwrap_or(1e-6),
            dist_threshold.unwrap_or(1e-4),
        )
    };

    if calculate_uv_area(uvs) < calc_area_threshold {
        return true;
    }

    let mut max_dist_sq = 0.0f32;
    for i in 0..uvs.len() {
        for j in (i + 1)..uvs.len() {
            let dist_sq = (uvs[i] - uvs[j]).length_squared();
            if dist_sq > max_dist_sq {
                max_dist_sq = dist_sq;
            }
        }
    }

    max_dist_sq < (calc_dist_threshold * calc_dist_threshold)
}

/// Check if an angle in radians is close to a multiple of 90 degrees (pi/2).
#[inline]
pub fn is_orthogonal_angle(angle_rad: f32, tolerance: f32) -> bool {
    let rem = angle_rad.abs() % FRAC_PI_2;
    rem < tolerance || (rem - FRAC_PI_2).abs() < tolerance
}

/// Calculate the Euler Z rotation angle (in radians) of a face's loop UVs.
///
/// Returns 0.0 for unrotated or standard axis-aligned faces (including right
/// triangles and trapezoids where at least one edge aligns orthogonally).
/// Returns non-zero angle theta in radians only when ALL valid edges of the face
/// are non-orthogonal (e.g., jmc2obj flowing liquid UVs rotated at 45°, 135°).
pub fn detect_uv_rotation(uvs: &[Vec2], tolerance: f32) -> f32 {
    let num_uvs = uvs.len();
    if num_uvs < 3 {
        return 0.0;
    }

    let mut valid_edges = Vec::with_capacity(num_uvs);
    for i in 0..num_uvs {
        let p_curr = uvs[i];
        let p_next = uvs[(i + 1) % num_uvs];
        let edge = p_next - p_curr;
        if edge.length() >= 1e-6 {
            let mut theta = edge.y.atan2(edge.x);
            while theta <= -PI {
                theta += 2.0 * PI;
            }
            while theta > PI {
                theta -= 2.0 * PI;
            }
            valid_edges.push((edge, theta));
        }
    }

    if valid_edges.is_empty() {
        return 0.0;
    }

    // If ANY edge is orthogonal, the face is built in an axis-aligned grid
    for (_edge, theta) in &valid_edges {
        if is_orthogonal_angle(*theta, tolerance) {
            return 0.0;
        }
    }

    let primary_theta = valid_edges[0].1;
    if is_orthogonal_angle(primary_theta, tolerance) {
        return 0.0;
    }

    primary_theta
}

/// Straighten a rotated polygon's UVs back to standard axis-aligned coordinates.
///
/// Rotates UVs by `-angle` around the UV geometric center.
/// Returns `(angle, was_straightened, straightened_uvs)`.
pub fn straighten_uv(uvs: &[Vec2], angle: Option<f32>) -> (f32, bool, Vec<Vec2>) {
    if uvs.len() < 3 {
        return (0.0, false, uvs.to_vec());
    }

    let ang = angle.unwrap_or_else(|| detect_uv_rotation(uvs, 1e-3));
    if ang.abs() < 1e-4 {
        return (0.0, false, uvs.to_vec());
    }

    let center = get_uv_center(uvs);
    let cos_t = (-ang).cos();
    let sin_t = (-ang).sin();

    let mut straightened = Vec::with_capacity(uvs.len());
    for uv in uvs {
        let dx = uv.x - center.x;
        let dy = uv.y - center.y;
        let new_x = center.x + (dx * cos_t - dy * sin_t);
        let new_y = center.y + (dx * sin_t + dy * cos_t);
        straightened.push(Vec2::new(new_x, new_y));
    }

    (ang, true, straightened)
}

/// Scale individual UV coordinates in place around their geometric center.
#[inline]
pub fn scale_uv(uvs: &[Vec2], scale_factor: f32) -> Vec<Vec2> {
    if uvs.is_empty() {
        return Vec::new();
    }
    let center = get_uv_center(uvs);
    uvs.iter()
        .map(|uv| center + (*uv - center) * scale_factor)
        .collect()
}

/// Normalize local UV rectangle into [0, 1] and return scale and location for `MC_Atlas_UV_Tiling`.
pub fn normalize_uv_for_atlas_tiling(
    uvs: &[Vec2],
    epsilon: f32,
) -> (Vec<Vec2>, [f32; 3], [f32; 3]) {
    if uvs.is_empty() {
        return (Vec::new(), [1.0, 1.0, 1.0], [0.0, 0.0, 0.0]);
    }

    let bounds = get_uv_bounds(uvs);
    let min_u = bounds.min.x;
    let max_u = bounds.max.x;
    let min_v = bounds.min.y;
    let max_v = bounds.max.y;
    let span_u = max_u - min_u;
    let span_v = max_v - min_v;

    let safe_span_u = if span_u > epsilon { span_u } else { 1.0 };
    let safe_span_v = if span_v > epsilon { span_v } else { 1.0 };

    let normalized = uvs
        .iter()
        .map(|uv| {
            let nx = if span_u > epsilon {
                (uv.x - min_u) / safe_span_u
            } else {
                0.0
            };
            let ny = if span_v > epsilon {
                (uv.y - min_v) / safe_span_v
            } else {
                0.0
            };
            Vec2::new(nx, ny)
        })
        .collect();

    let scale = [safe_span_u, safe_span_v, 1.0];
    let location = [
        min_u + (safe_span_u - 1.0) * 0.5,
        min_v + (safe_span_v - 1.0) * 0.5,
        0.0,
    ];

    (normalized, scale, location)
}

/// Return whether local UVs need shader-side wrapping to fit an Atlas cell.
#[inline]
pub fn uv_requires_atlas_tiling(uvs: &[Vec2], epsilon: f32) -> bool {
    if uvs.is_empty() {
        return false;
    }
    let bounds = get_uv_bounds(uvs);
    bounds.min.x < -epsilon
        || bounds.max.x > 1.0 + epsilon
        || bounds.min.y < -epsilon
        || bounds.max.y > 1.0 + epsilon
}

/// Apply the Atlas tiling node's affine mapping to normalized local UV.
#[inline]
pub fn restore_atlas_tiling_uv(
    u: f32,
    v: f32,
    scale: [f32; 3],
    location: [f32; 3],
    rotation: f32,
) -> (f32, f32) {
    let sx = scale[0];
    let sy = scale[1];
    let lx = location[0];
    let ly = location[1];
    let mut x = sx * (u - 0.5);
    let mut y = sy * (v - 0.5);
    if rotation.abs() > 1e-8 {
        let cos_t = rotation.cos();
        let sin_t = rotation.sin();
        let rx = x * cos_t - y * sin_t;
        let ry = x * sin_t + y * cos_t;
        x = rx;
        y = ry;
    }
    (x + lx + 0.5, y + ly + 0.5)
}

#[derive(Debug, Clone, Copy)]
struct FaceAxisCandidate {
    top_diff: f32,
    base_diff: f32,
    t1_idx: usize,
    t2_idx: usize,
}

#[inline]
fn evaluate_face_axis(verts: &[Vec3; 4], normal: Vec3, up_axis: Vec3) -> Option<FaceAxisCandidate> {
    if normal.length_squared() < 1e-12 {
        return None;
    }
    let proj_up = up_axis - up_axis.dot(normal) * normal;
    if proj_up.length_squared() < 1e-8 {
        return None;
    }
    let proj_up = proj_up.normalize();

    let mut heights = [
        (0usize, verts[0].dot(proj_up)),
        (1usize, verts[1].dot(proj_up)),
        (2usize, verts[2].dot(proj_up)),
        (3usize, verts[3].dot(proj_up)),
    ];
    // Sort ascending by height
    heights.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

    let base_diff = (heights[1].1 - heights[0].1).abs();
    let top_diff = heights[3].1 - heights[2].1;

    Some(FaceAxisCandidate {
        top_diff,
        base_diff,
        t1_idx: heights[2].0,
        t2_idx: heights[3].0,
    })
}

/// Check and repair inverted fluid UV on a single 4-vertex quad face.
///
/// Returns `true` if UV coordinates were repaired (swapped), `false` otherwise.
pub fn repair_quad_fluid_uv(
    verts: &[Vec3; 4],
    uvs: &mut [Vec2; 4],
    normal: Option<Vec3>,
    force: bool,
    min_slope_threshold: f32,
) -> bool {
    let norm = match normal {
        Some(n) if n.length_squared() >= 1e-12 => n.normalize(),
        _ => {
            let n = (verts[1] - verts[0]).cross(verts[2] - verts[0])
                + (verts[2] - verts[0]).cross(verts[3] - verts[0]);
            if n.length_squared() < 1e-12 {
                return false;
            }
            n.normalize()
        }
    };

    // Candidates for up-axis: Y-up (Minecraft OBJ) and Z-up (Blender native)
    let eval_y = evaluate_face_axis(verts, norm, Vec3::Y);
    let eval_z = evaluate_face_axis(verts, norm, Vec3::Z);

    let best_eval = match (eval_y, eval_z) {
        (Some(y), Some(z)) => {
            let score_y = y.top_diff - y.base_diff;
            let score_z = z.top_diff - z.base_diff;
            if score_y >= score_z {
                y
            } else {
                z
            }
        }
        (Some(y), None) => y,
        (None, Some(z)) => z,
        (None, None) => return false,
    };

    if best_eval.top_diff < min_slope_threshold && !force {
        return false;
    }

    let t1 = best_eval.t1_idx;
    let t2 = best_eval.t2_idx;

    let uv1_y = uvs[t1].y;
    let uv2_y = uvs[t2].y;

    let uv_v_diff = uv2_y - uv1_y;
    let is_inverted = uv_v_diff < -1e-5;

    if is_inverted || (force && best_eval.top_diff >= min_slope_threshold) {
        uvs[t1].y = uv2_y;
        uvs[t2].y = uv1_y;
        true
    } else {
        false
    }
}

/// Batch repair inverted fluid UVs across flat slices of quad vertices and UVs.
///
/// verts_flat: N * 4 * 3 floats.
/// uvs_flat: N * 4 * 2 floats.
/// normals_flat: Optional N * 3 floats (precomputed face normals).
pub fn batch_repair_fluid_uv(
    verts_flat: &[f32],
    uvs_flat: &mut [f32],
    normals_flat: Option<&[f32]>,
    force: bool,
    min_slope_threshold: f32,
) -> usize {
    let num_faces = verts_flat.len() / 12;
    if num_faces == 0 || uvs_flat.len() < num_faces * 8 {
        return 0;
    }

    let mut repaired_count = 0;
    for f in 0..num_faces {
        let v_off = f * 12;
        let uv_off = f * 8;
        let verts = [
            Vec3::new(verts_flat[v_off], verts_flat[v_off + 1], verts_flat[v_off + 2]),
            Vec3::new(verts_flat[v_off + 3], verts_flat[v_off + 4], verts_flat[v_off + 5]),
            Vec3::new(verts_flat[v_off + 6], verts_flat[v_off + 7], verts_flat[v_off + 8]),
            Vec3::new(verts_flat[v_off + 9], verts_flat[v_off + 10], verts_flat[v_off + 11]),
        ];
        let mut uvs = [
            Vec2::new(uvs_flat[uv_off], uvs_flat[uv_off + 1]),
            Vec2::new(uvs_flat[uv_off + 2], uvs_flat[uv_off + 3]),
            Vec2::new(uvs_flat[uv_off + 4], uvs_flat[uv_off + 5]),
            Vec2::new(uvs_flat[uv_off + 6], uvs_flat[uv_off + 7]),
        ];

        let normal = normals_flat.and_then(|ns| {
            if ns.len() >= (f + 1) * 3 {
                Some(Vec3::new(ns[f * 3], ns[f * 3 + 1], ns[f * 3 + 2]))
            } else {
                None
            }
        });

        if repair_quad_fluid_uv(&verts, &mut uvs, normal, force, min_slope_threshold) {
            uvs_flat[uv_off] = uvs[0].x;
            uvs_flat[uv_off + 1] = uvs[0].y;
            uvs_flat[uv_off + 2] = uvs[1].x;
            uvs_flat[uv_off + 3] = uvs[1].y;
            uvs_flat[uv_off + 4] = uvs[2].x;
            uvs_flat[uv_off + 5] = uvs[2].y;
            uvs_flat[uv_off + 6] = uvs[3].x;
            uvs_flat[uv_off + 7] = uvs[3].y;
            repaired_count += 1;
        }
    }

    repaired_count
}

/// Get Minecraft-standard UV coordinates for top/bottom fluid faces.
pub fn get_fluid_top_uvs(is_flowing: bool, rotation: f32) -> [Vec2; 4] {
    if !is_flowing {
        return [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];
    }

    let base = [
        Vec2::new(0.25, 0.25),
        Vec2::new(0.25, 0.75),
        Vec2::new(0.75, 0.75),
        Vec2::new(0.75, 0.25),
    ];

    if rotation.abs() < 1e-4 {
        return base;
    }

    // In MC coordinate space, rotating Blender UV by +rotation corresponds to rotating MC UV by -rotation:
    let cos_t = (-rotation).cos();
    let sin_t = (-rotation).sin();

    [
        Vec2::new(
            0.5 + ((base[0].x - 0.5) * cos_t - (base[0].y - 0.5) * sin_t),
            0.5 + ((base[0].x - 0.5) * sin_t + (base[0].y - 0.5) * cos_t),
        ),
        Vec2::new(
            0.5 + ((base[1].x - 0.5) * cos_t - (base[1].y - 0.5) * sin_t),
            0.5 + ((base[1].x - 0.5) * sin_t + (base[1].y - 0.5) * cos_t),
        ),
        Vec2::new(
            0.5 + ((base[2].x - 0.5) * cos_t - (base[2].y - 0.5) * sin_t),
            0.5 + ((base[2].x - 0.5) * sin_t + (base[2].y - 0.5) * cos_t),
        ),
        Vec2::new(
            0.5 + ((base[3].x - 0.5) * cos_t - (base[3].y - 0.5) * sin_t),
            0.5 + ((base[3].x - 0.5) * sin_t + (base[3].y - 0.5) * cos_t),
        ),
    ]
}

/// Get Minecraft/Mineways-standard UV coordinates for vertical/sloped fluid side faces.
pub fn get_fluid_side_uvs(h_left_top: f32, h_right_top: f32) -> [Vec2; 4] {
    [
        Vec2::new(0.0, (1.0 - h_left_top) * 0.5),
        Vec2::new(0.0, 0.5),
        Vec2::new(0.5, 0.5),
        Vec2::new(0.5, (1.0 - h_right_top) * 0.5),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_uv_area_quad() {
        let quad = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        assert!((calculate_uv_area(&quad) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_uv_bounds_and_center() {
        let quad = vec![
            Vec2::new(0.2, 0.3),
            Vec2::new(0.8, 0.3),
            Vec2::new(0.8, 0.7),
            Vec2::new(0.2, 0.7),
        ];
        let bounds = get_uv_bounds(&quad);
        assert!((bounds.min.x - 0.2).abs() < 1e-6);
        assert!((bounds.max.x - 0.8).abs() < 1e-6);
        assert!((bounds.min.y - 0.3).abs() < 1e-6);
        assert!((bounds.max.y - 0.7).abs() < 1e-6);
        assert!((bounds.width() - 0.6).abs() < 1e-6);
        assert!((bounds.height() - 0.4).abs() < 1e-6);

        let center = get_uv_center(&quad);
        assert!((center.x - 0.5).abs() < 1e-6);
        assert!((center.y - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_uv_collapse_detection() {
        let collapsed_line = vec![
            Vec2::new(0.5, 0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(0.5, 0.5),
        ];
        assert!(is_uv_collapsed(&collapsed_line, None, None, None));

        let valid_quad = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        assert!(!is_uv_collapsed(&valid_quad, None, None, None));
    }

    #[test]
    fn test_orthogonal_angle_and_rotation() {
        assert!(is_orthogonal_angle(0.0, 1e-3));
        assert!(is_orthogonal_angle(FRAC_PI_2, 1e-3));
        assert!(is_orthogonal_angle(PI, 1e-3));
        assert!(is_orthogonal_angle(-FRAC_PI_2, 1e-3));
        assert!(!is_orthogonal_angle(FRAC_PI_2 / 2.0, 1e-3));

        // 45 degree rotated diamond UV
        let rad45 = core::f32::consts::FRAC_PI_4;
        let rot_quad = vec![
            Vec2::new(0.5, 0.0),
            Vec2::new(1.0, 0.5),
            Vec2::new(0.5, 1.0),
            Vec2::new(0.0, 0.5),
        ];
        let detected = detect_uv_rotation(&rot_quad, 1e-3);
        assert!((detected.abs() - rad45).abs() < 1e-4);

        let (angle, straightened, new_uvs) = straighten_uv(&rot_quad, None);
        assert!(straightened);
        assert!((angle.abs() - rad45).abs() < 1e-4);
        assert_eq!(new_uvs.len(), 4);
    }

    #[test]
    fn test_scale_uv() {
        let quad = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];
        let scaled = scale_uv(&quad, 0.5);
        assert!((scaled[0].x - 0.25).abs() < 1e-6);
        assert!((scaled[0].y - 0.25).abs() < 1e-6);
        assert!((scaled[2].x - 0.75).abs() < 1e-6);
        assert!((scaled[2].y - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_repair_inverted_fluid_uv() {
        // Quad face: bottom is Y=0, top left is Y=0.2, top right is Y=0.8
        let verts = [
            Vec3::new(0.0, 0.0, 1.0),  // Bottom right
            Vec3::new(0.0, 0.0, 0.0),  // Bottom left
            Vec3::new(0.0, 0.2, 0.0),  // Top left (low: 0.2)
            Vec3::new(0.0, 0.8, 1.0),  // Top right (high: 0.8)
        ];
        let mut uvs = [
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.8), // Inverted
            Vec2::new(1.0, 0.2), // Inverted
        ];

        let repaired = repair_quad_fluid_uv(&verts, &mut uvs, None, false, 0.005);
        assert!(repaired);
        assert!((uvs[2].y - 0.2).abs() < 1e-5);
        assert!((uvs[3].y - 0.8).abs() < 1e-5);
        assert!((uvs[2].x - 0.0).abs() < 1e-5);
        assert!((uvs[3].x - 1.0).abs() < 1e-5);

        // Non-inverted face should not be modified
        let mut valid_uvs = [
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 0.2),
            Vec2::new(1.0, 0.8),
        ];
        let repaired_again = repair_quad_fluid_uv(&verts, &mut valid_uvs, None, false, 0.005);
        assert!(!repaired_again);
        assert!((valid_uvs[2].y - 0.2).abs() < 1e-5);
        assert!((valid_uvs[3].y - 0.8).abs() < 1e-5);
    }

    #[test]
    fn test_fluid_top_and_side_uvs() {
        let top_stationary = get_fluid_top_uvs(false, 0.0);
        assert_eq!(top_stationary[0], Vec2::new(0.0, 0.0));
        assert_eq!(top_stationary[2], Vec2::new(1.0, 1.0));

        let top_flowing = get_fluid_top_uvs(true, 0.0);
        assert_eq!(top_flowing[0], Vec2::new(0.25, 0.25));
        assert_eq!(top_flowing[2], Vec2::new(0.75, 0.75));

        let side = get_fluid_side_uvs(0.8, 0.2);
        assert!((side[0].y - (1.0 - 0.8) * 0.5).abs() < 1e-6);
        assert!((side[3].y - (1.0 - 0.2) * 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_uv_area_2d_and_collapse_2d() {
        let quad_uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert!((calculate_uv_area_2d(&quad_uvs) - 1.0).abs() < 1e-6);
        assert!(!is_uv_collapsed_2d(&quad_uvs, Some([1.0 / 16.0, 1.0 / 16.0])));

        let collapsed_line = [[0.1, 0.2], [0.1, 0.2], [0.1, 0.2], [0.1, 0.2]];
        assert_eq!(calculate_uv_area_2d(&collapsed_line), 0.0);
        assert!(is_uv_collapsed_2d(&collapsed_line, Some([1.0 / 16.0, 1.0 / 16.0])));
    }
}

