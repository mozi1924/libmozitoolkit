//! Minecraft fluid UV calculations and inverted slanted UV repairs.

use glam::{Vec2, Vec3};

/// Computes canonical Minecraft UV coordinates for top and bottom fluid quad faces.
///
/// Flowing fluids sample a 16x16 window (`[0.25, 0.75]`) inside the 32x32 sprite,
/// centered at `(0.5, 0.5)`, with rotation baked directly into the coordinates.
/// Stationary source pools sample the standard full `[0, 1]` sprite.
pub fn get_fluid_top_uvs(is_flowing: bool, rotation: f32) -> [[f32; 2]; 4] {
    if !is_flowing {
        return [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
    }

    let base_uvs = [[0.25f32, 0.25f32], [0.25, 0.75], [0.75, 0.75], [0.75, 0.25]];
    if rotation.abs() < 1e-4 {
        return base_uvs;
    }

    // In Minecraft coordinate space where Y is inverted relative to standard UV space,
    // rotating UV by -rotation aligns with the fluid stream flow direction.
    let cos_t = (-rotation).cos();
    let sin_t = (-rotation).sin();

    let mut rotated = [[0.0f32; 2]; 4];
    for i in 0..4 {
        let du = base_uvs[i][0] - 0.5;
        let dv = base_uvs[i][1] - 0.5;
        rotated[i][0] = 0.5 + (du * cos_t - dv * sin_t);
        rotated[i][1] = 0.5 + (du * sin_t + dv * cos_t);
    }
    rotated
}

/// Get Minecraft-standard UV coordinates for top/bottom fluid faces as `[Vec2; 4]`.
#[inline]
pub fn get_fluid_top_uvs_vec2(is_flowing: bool, rotation: f32) -> [Vec2; 4] {
    let arr = get_fluid_top_uvs(is_flowing, rotation);
    [
        Vec2::new(arr[0][0], arr[0][1]),
        Vec2::new(arr[1][0], arr[1][1]),
        Vec2::new(arr[2][0], arr[2][1]),
        Vec2::new(arr[3][0], arr[3][1]),
    ]
}

/// Computes Minecraft / Mineways-standard non-collapsed UV coordinates for vertical/slanted fluid side faces.
///
/// Side faces sample the `[0.0, 0.5]` quadrant of the 32x32 sprite, mapping
/// proportional 1-block height to 16 pixels.
#[inline]
pub fn get_fluid_side_uvs(h_left_top: f32, h_right_top: f32) -> [[f32; 2]; 4] {
    [
        [0.0, (1.0 - h_left_top) * 0.5],
        [0.0, 0.5],
        [0.5, 0.5],
        [0.5, (1.0 - h_right_top) * 0.5],
    ]
}

/// Get Minecraft/Mineways-standard UV coordinates for vertical/sloped fluid side faces as `[Vec2; 4]`.
#[inline]
pub fn get_fluid_side_uvs_vec2(h_left_top: f32, h_right_top: f32) -> [Vec2; 4] {
    let arr = get_fluid_side_uvs(h_left_top, h_right_top);
    [
        Vec2::new(arr[0][0], arr[0][1]),
        Vec2::new(arr[1][0], arr[1][1]),
        Vec2::new(arr[2][0], arr[2][1]),
        Vec2::new(arr[3][0], arr[3][1]),
    ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_still_top_uvs() {
        let uvs = get_fluid_top_uvs(false, 0.0);
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [0.0, 1.0]);
        assert_eq!(uvs[2], [1.0, 1.0]);
        assert_eq!(uvs[3], [1.0, 0.0]);

        let uvs_v2 = get_fluid_top_uvs_vec2(false, 0.0);
        assert_eq!(uvs_v2[0], Vec2::new(0.0, 0.0));
        assert_eq!(uvs_v2[2], Vec2::new(1.0, 1.0));
    }

    #[test]
    fn test_flowing_top_uvs() {
        let uvs = get_fluid_top_uvs(true, 0.0);
        assert_eq!(uvs[0], [0.25, 0.25]);
        assert_eq!(uvs[2], [0.75, 0.75]);

        let uvs_v2 = get_fluid_top_uvs_vec2(true, 0.0);
        assert_eq!(uvs_v2[0], Vec2::new(0.25, 0.25));
        assert_eq!(uvs_v2[2], Vec2::new(0.75, 0.75));
    }

    #[test]
    fn test_side_uvs() {
        let uvs = get_fluid_side_uvs(1.0, 0.5);
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [0.0, 0.5]);
        assert_eq!(uvs[2], [0.5, 0.5]);
        assert_eq!(uvs[3], [0.5, 0.25]);

        let side_v2 = get_fluid_side_uvs_vec2(0.8, 0.2);
        assert!((side_v2[0].y - (1.0 - 0.8) * 0.5).abs() < 1e-6);
        assert!((side_v2[3].y - (1.0 - 0.2) * 0.5).abs() < 1e-6);
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
}
