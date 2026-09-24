//! # Adaptive Pixel Split & Quad Subdivider
//!
//! Provides pixel-density aware quad face subdivision with bilinear interpolation
//! of all vertex and corner attributes (positions, normals, UVs, colors, custom layers, bone deform weights).

use alloc::vec::Vec;
use core::cmp::max;

use crate::attributes::{AttributeData, MeshAttribute};

use crate::geometry::Aabb2d;
use crate::mesh::MeshData;

/// Calculates target (cols, rows) subdivisions for a quad face based on texture resolution and UV span.
///
/// Computes exact UV edge vector lengths in texture pixel space, ensuring 1:1 physical pixel alignment
/// for Atlas sub-regions, non-square tiles, rotated UVs, and long vertical animated strips.
pub fn calculate_face_target_grid(
    uvs: &[[f32; 2]],
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> (u32, u32) {
    if uvs.is_empty() || tex_w == 0 || tex_h == 0 {
        return (1, 1);
    }

    let ppf = if pixels_per_face <= 0.0 { 1.0 } else { pixels_per_face };
    let max_sub = max(1, max_subdivisions);

    // If we have 4 quad corner UVs [uv0, uv1, uv2, uv3]:
    let (u_pixels, v_pixels) = if uvs.len() >= 4 {
        let uv0 = uvs[0];
        let uv1 = uvs[1];
        let uv2 = uvs[2];
        let uv3 = uvs[3];

        // Bottom edge (0 -> 1) and top edge (3 -> 2)
        let du0 = uv1[0] - uv0[0];
        let dv0 = uv1[1] - uv0[1];
        let du1 = uv2[0] - uv3[0];
        let dv1 = uv2[1] - uv3[1];

        // Left edge (0 -> 3) and right edge (1 -> 2)
        let du2 = uv3[0] - uv0[0];
        let dv2 = uv3[1] - uv0[1];
        let du3 = uv2[0] - uv1[0];
        let dv3 = uv2[1] - uv1[1];

        let px_u0 = ((du0 * tex_w as f32).powi(2) + (dv0 * tex_h as f32).powi(2)).sqrt();
        let px_u1 = ((du1 * tex_w as f32).powi(2) + (dv1 * tex_h as f32).powi(2)).sqrt();
        let px_v0 = ((du2 * tex_w as f32).powi(2) + (dv2 * tex_h as f32).powi(2)).sqrt();
        let px_v1 = ((du3 * tex_w as f32).powi(2) + (dv3 * tex_h as f32).powi(2)).sqrt();

        (0.5 * (px_u0 + px_u1), 0.5 * (px_v0 + px_v1))
    } else {
        // Fallback for non-quad: compute bounding box span
        let aabb = Aabb2d::from_points(uvs);
        let u_span = (aabb.max.x - aabb.min.x).abs();
        let v_span = (aabb.max.y - aabb.min.y).abs();
        (u_span * tex_w as f32, v_span * tex_h as f32)
    };

    // Anti-explosion defense for long vertical animated strips (e.g. 16x512)
    let effective_v_pixels = if tex_h > tex_w && (tex_h % tex_w == 0) && v_pixels > (tex_w as f32 * 1.5) {
        tex_w as f32
    } else {
        v_pixels
    };

    let cols = ((u_pixels / ppf).round() as u32).clamp(1, max_sub);
    let rows = ((effective_v_pixels / ppf).round() as u32).clamp(1, max_sub);

    (cols, rows)
}

/// Calculates non-uniform [0, 1] parameter cut factors along U and V axes of a quad face,
/// directly snapping interior cuts to the integer pixel grid lines of the texture image.
/// Calculates non-uniform [0, 1] parameter cut factors along U and V axes of a quad face,
/// directly snapping interior cuts to the integer pixel grid lines of the texture image.
///
/// Returns `(u_factors: Vec<f32>, v_factors: Vec<f32>)` where each factor list starts with 0.0 and ends with 1.0.
pub fn calculate_pixel_grid_cut_factors(
    uvs: &[[f32; 2]],
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> (Vec<f32>, Vec<f32>) {
    if uvs.len() < 4 || tex_w == 0 || tex_h == 0 {
        return (vec![0.0, 1.0], vec![0.0, 1.0]);
    }

    let step = if pixels_per_face <= 0.0 { 1.0 } else { pixels_per_face };
    let max_sub = max_subdivisions.max(1) as usize;

    // Convert UVs to pixel space
    let p0 = [uvs[0][0] * tex_w as f32, uvs[0][1] * tex_h as f32];
    let p1 = [uvs[1][0] * tex_w as f32, uvs[1][1] * tex_h as f32];
    let p2 = [uvs[2][0] * tex_w as f32, uvs[2][1] * tex_h as f32];
    let p3 = [uvs[3][0] * tex_w as f32, uvs[3][1] * tex_h as f32];

    // Helper to find integer grid line cuts along an edge
    fn get_edge_cuts(pa: [f32; 2], pb: [f32; 2], step: f32, max_cuts: usize) -> Vec<f32> {
        let dx = pb[0] - pa[0];
        let dy = pb[1] - pa[1];

        // Determine dominant pixel axis
        let (start, end) = if dx.abs() >= dy.abs() {
            (pa[0], pb[0])
        } else {
            (pa[1], pb[1])
        };

        let span = end - start;
        if span.abs() < 1e-4 {
            return vec![0.0, 1.0];
        }

        let mut cuts = Vec::new();
        cuts.push(0.0);

        if span > 0.0 {
            // Increasing direction
            let mut k = ((start / step) + 1e-4).ceil() * step;
            while k < end - 1e-4 {
                let factor = (k - start) / span;
                if factor > 1e-4 && factor < (1.0 - 1e-4) {
                    cuts.push(factor);
                }
                k += step;
                if cuts.len() >= max_cuts {
                    break;
                }
            }
        } else {
            // Decreasing direction
            let mut k = ((start / step) - 1e-4).floor() * step;
            while k > end + 1e-4 {
                let factor = (k - start) / span;
                if factor > 1e-4 && factor < (1.0 - 1e-4) {
                    cuts.push(factor);
                }
                k -= step;
                if cuts.len() >= max_cuts {
                    break;
                }
            }
        }

        cuts.push(1.0);
        cuts
    }

    // Compute cuts along U (bottom edge p0->p1 and top edge p3->p2)
    let u_cuts_bot = get_edge_cuts(p0, p1, step, max_sub);
    let u_cuts_top = get_edge_cuts(p3, p2, step, max_sub);
    let u_cuts = if u_cuts_bot.len() >= u_cuts_top.len() {
        u_cuts_bot
    } else {
        u_cuts_top
    };

    // Compute cuts along V (left edge p0->p3 and right edge p1->p2)
    let v_cuts_left = get_edge_cuts(p0, p3, step, max_sub);
    let v_cuts_right = get_edge_cuts(p1, p2, step, max_sub);
    let v_cuts = if v_cuts_left.len() >= v_cuts_right.len() {
        v_cuts_left
    } else {
        v_cuts_right
    };

    (u_cuts, v_cuts)
}

/// Bilinear interpolation helper for scalar / vector types.
#[inline]
pub fn interpolate_bilinear_2d(c0: [f32; 2], c1: [f32; 2], c2: [f32; 2], c3: [f32; 2], u: f32, v: f32) -> [f32; 2] {
    let u_inv = 1.0 - u;
    let v_inv = 1.0 - v;
    [
        u_inv * v_inv * c0[0] + u * v_inv * c1[0] + u * v * c2[0] + u_inv * v * c3[0],
        u_inv * v_inv * c0[1] + u * v_inv * c1[1] + u * v * c2[1] + u_inv * v * c3[1],
    ]
}

#[inline]
pub fn interpolate_bilinear_3d(c0: [f32; 3], c1: [f32; 3], c2: [f32; 3], c3: [f32; 3], u: f32, v: f32) -> [f32; 3] {
    let u_inv = 1.0 - u;
    let v_inv = 1.0 - v;
    [
        u_inv * v_inv * c0[0] + u * v_inv * c1[0] + u * v * c2[0] + u_inv * v * c3[0],
        u_inv * v_inv * c0[1] + u * v_inv * c1[1] + u * v * c2[1] + u_inv * v * c3[1],
        u_inv * v_inv * c0[2] + u * v_inv * c1[2] + u * v * c2[2] + u_inv * v * c3[2],
    ]
}

#[inline]
pub fn interpolate_bilinear_4d(c0: [f32; 4], c1: [f32; 4], c2: [f32; 4], c3: [f32; 4], u: f32, v: f32) -> [f32; 4] {
    let u_inv = 1.0 - u;
    let v_inv = 1.0 - v;
    [
        u_inv * v_inv * c0[0] + u * v_inv * c1[0] + u * v * c2[0] + u_inv * v * c3[0],
        u_inv * v_inv * c0[1] + u * v_inv * c1[1] + u * v * c2[1] + u_inv * v * c3[1],
        u_inv * v_inv * c0[2] + u * v_inv * c1[2] + u * v * c2[2] + u_inv * v * c3[2],
        u_inv * v_inv * c0[3] + u * v_inv * c1[3] + u * v * c2[3] + u_inv * v * c3[3],
    ]
}

/// Result of slicing a single face by the 2D texture pixel grid.
#[derive(Debug, Clone, Default)]
pub struct SlicedFaceResult {
    /// 3D vertex positions for all unique vertices.
    pub positions: Vec<[f32; 3]>,
    /// Exact UV coordinates for each unique vertex (aligned to pixel grid).
    pub uvs: Vec<[f32; 2]>,
    /// Sliced sub-faces, each represented as a list of vertex indices (e.g. 4 for quads, 3 for triangles).
    pub faces: Vec<Vec<u32>>,
    /// Parametric coordinates (s, t) in [0, 1] relative to the original face corners,
    /// used by DCCs to bilinearly interpolate bone deform weights, colors, and custom layers.
    pub param_coords: Vec<[f32; 2]>,
}

/// Inverts a 2D bilinear map from UV space to parametric coordinates (s, t) in [0, 1]^2.
pub fn invert_quad_bilinear(
    uv0: [f32; 2],
    uv1: [f32; 2],
    uv2: [f32; 2],
    uv3: [f32; 2],
    u: f32,
    v: f32,
) -> [f32; 2] {
    let a = uv0;
    let b = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
    let c = [uv3[0] - uv0[0], uv3[1] - uv0[1]];
    let d = [
        uv0[0] - uv1[0] + uv2[0] - uv3[0],
        uv0[1] - uv1[1] + uv2[1] - uv3[1],
    ];

    let p = [u - a[0], v - a[1]];

    // If d is near zero, the quad is a parallelogram (rectangle, diamond, or sheared parallelogram)
    if d[0].abs() < 1e-5 && d[1].abs() < 1e-5 {
        let det = b[0] * c[1] - b[1] * c[0];
        if det.abs() > 1e-6 {
            let s = (p[0] * c[1] - p[1] * c[0]) / det;
            let t = (b[0] * p[1] - b[1] * p[0]) / det;
            return [s.clamp(0.0, 1.0), t.clamp(0.0, 1.0)];
        }
    }

    // General quadratic equation in t:
    let a_quad = c[0] * d[1] - c[1] * d[0];
    let b_quad = c[0] * b[1] - c[1] * b[0] + p[0] * d[1] - p[1] * d[0];
    let c_quad = p[0] * b[1] - p[1] * b[0];

    let t = if a_quad.abs() < 1e-6 {
        if b_quad.abs() > 1e-6 {
            -c_quad / b_quad
        } else {
            0.5
        }
    } else {
        let disc = b_quad * b_quad - 4.0 * a_quad * c_quad;
        if disc >= 0.0 {
            let sqrt_disc = disc.sqrt();
            let t1 = (-b_quad + sqrt_disc) / (2.0 * a_quad);
            let t2 = (-b_quad - sqrt_disc) / (2.0 * a_quad);
            if (0.0..=1.0).contains(&t1) {
                t1
            } else if (0.0..=1.0).contains(&t2) {
                t2
            } else if (t1 - 0.5).abs() < (t2 - 0.5).abs() {
                t1
            } else {
                t2
            }
        } else {
            -b_quad / (2.0 * a_quad)
        }
    };
    let t_clamped = t.clamp(0.0, 1.0);

    let denom_x = b[0] + t_clamped * d[0];
    let denom_y = b[1] + t_clamped * d[1];
    let num_x = p[0] - t_clamped * c[0];
    let num_y = p[1] - t_clamped * c[1];

    let s = if denom_x.abs() >= denom_y.abs() && denom_x.abs() > 1e-6 {
        num_x / denom_x
    } else if denom_y.abs() > 1e-6 {
        num_y / denom_y
    } else {
        0.5
    };

    [s.clamp(0.0, 1.0), t_clamped]
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point2D {
    x: f32,
    y: f32,
}

fn polygon_signed_area_2d(poly: &[Point2D]) -> f32 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    let n = poly.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += poly[i].x * poly[j].y - poly[j].x * poly[i].y;
    }
    0.5 * area
}

fn clean_polygon_2d(poly: &[Point2D]) -> Vec<Point2D> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let mut res: Vec<Point2D> = Vec::with_capacity(poly.len());
    for &pt in poly {
        if let Some(&last) = res.last() {
            if (pt.x - last.x).abs() < 1e-5 && (pt.y - last.y).abs() < 1e-5 {
                continue;
            }
        }
        res.push(pt);
    }
    if let (Some(&first), Some(&last)) = (res.first(), res.last()) {
        if res.len() > 1 && (first.x - last.x).abs() < 1e-5 && (first.y - last.y).abs() < 1e-5 {
            res.pop();
        }
    }
    if res.len() < 3 || polygon_signed_area_2d(&res).abs() < 1e-6 {
        Vec::new()
    } else {
        res
    }
}

fn clip_polygon_halfplane_2d(
    poly: &[Point2D],
    is_vertical: bool,
    val: f32,
    keep_greater: bool,
) -> Vec<Point2D> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n + 2);
    for i in 0..n {
        let p1 = poly[i];
        let p2 = poly[(i + 1) % n];
        let d1 = if is_vertical { p1.x - val } else { p1.y - val };
        let d2 = if is_vertical { p2.x - val } else { p2.y - val };

        let in1 = if keep_greater { d1 >= -1e-5 } else { d1 <= 1e-5 };

        if in1 {
            out.push(p1);
        }

        if (d1 > 1e-5 && d2 < -1e-5) || (d1 < -1e-5 && d2 > 1e-5) {
            let denom = if is_vertical { p2.x - p1.x } else { p2.y - p1.y };
            let t = if denom.abs() > 1e-6 {
                ((val - if is_vertical { p1.x } else { p1.y }) / denom).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let mut mid = Point2D {
                x: p1.x + t * (p2.x - p1.x),
                y: p1.y + t * (p2.y - p1.y),
            };
            if is_vertical {
                mid.x = val;
            } else {
                mid.y = val;
            }
            out.push(mid);
        }
    }
    clean_polygon_2d(&out)
}

/// Slices a 2D/3D polygon strictly along the 2D texture pixel grid lines (X = 1, 2... and Y = 1, 2...).
///
/// For rotated, translated, or scaled UVs, interior faces are exact 1x1 square pixels in UV space,
/// and cut lines on the 3D mesh naturally match the slanted orientation of the texture.
pub fn slice_face_by_pixel_grid(
    positions: &[[f32; 3]],
    uvs: &[[f32; 2]],
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> SlicedFaceResult {
    let mut default_res = SlicedFaceResult::default();
    if positions.len() < 3 || uvs.len() < 3 || tex_w == 0 || tex_h == 0 {
        default_res.positions = positions.to_vec();
        default_res.uvs = uvs.to_vec();
        default_res.faces = vec![(0..positions.len() as u32).collect()];
        default_res.param_coords = vec![[0.0, 0.0]; positions.len()];
        return default_res;
    }

    let step = if pixels_per_face <= 0.0 { 1.0 } else { pixels_per_face };

    // Convert polygon to pixel space
    let mut initial_poly: Vec<Point2D> = uvs
        .iter()
        .map(|uv| Point2D {
            x: uv[0] * tex_w as f32,
            y: uv[1] * tex_h as f32,
        })
        .collect();

    // Ensure CCW orientation
    if polygon_signed_area_2d(&initial_poly) < 0.0 {
        initial_poly.reverse();
    }

    // Determine bounding box in pixel space
    let min_x = initial_poly.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = initial_poly.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = initial_poly.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = initial_poly.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

    // Collect vertical and horizontal grid lines
    let mut x_lines = Vec::new();
    let mut k_x = ((min_x / step) + 1e-4).ceil() * step;
    while k_x < max_x - 1e-4 {
        x_lines.push(k_x);
        k_x += step;
        if x_lines.len() >= max_subdivisions as usize {
            break;
        }
    }

    let mut y_lines = Vec::new();
    let mut k_y = ((min_y / step) + 1e-4).ceil() * step;
    while k_y < max_y - 1e-4 {
        y_lines.push(k_y);
        k_y += step;
        if y_lines.len() >= max_subdivisions as usize {
            break;
        }
    }

    // If no grid lines intersect the polygon, return original face
    if x_lines.is_empty() && y_lines.is_empty() {
        default_res.positions = positions.to_vec();
        default_res.uvs = uvs.to_vec();
        default_res.faces = vec![(0..positions.len() as u32).collect()];
        default_res.param_coords = match positions.len() {
            4 => vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            _ => vec![[0.0, 0.0]; positions.len()],
        };
        return default_res;
    }

    // Slice along vertical lines X
    let mut current_polys = vec![initial_poly];
    for x_val in x_lines {
        let mut next_polys = Vec::with_capacity(current_polys.len() * 2);
        for poly in current_polys {
            let px_min = poly.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
            let px_max = poly.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
            if x_val <= px_min + 1e-5 || x_val >= px_max - 1e-5 {
                next_polys.push(poly);
            } else {
                let neg = clip_polygon_halfplane_2d(&poly, true, x_val, false);
                let pos = clip_polygon_halfplane_2d(&poly, true, x_val, true);
                if neg.len() >= 3 && polygon_signed_area_2d(&neg).abs() > 1e-6 {
                    next_polys.push(neg);
                }
                if pos.len() >= 3 && polygon_signed_area_2d(&pos).abs() > 1e-6 {
                    next_polys.push(pos);
                }
            }
        }
        current_polys = next_polys;
    }

    // Slice along horizontal lines Y
    for y_val in y_lines {
        let mut next_polys = Vec::with_capacity(current_polys.len() * 2);
        for poly in current_polys {
            let py_min = poly.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
            let py_max = poly.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
            if y_val <= py_min + 1e-5 || y_val >= py_max - 1e-5 {
                next_polys.push(poly);
            } else {
                let neg = clip_polygon_halfplane_2d(&poly, false, y_val, false);
                let pos = clip_polygon_halfplane_2d(&poly, false, y_val, true);
                if neg.len() >= 3 && polygon_signed_area_2d(&neg).abs() > 1e-6 {
                    next_polys.push(neg);
                }
                if pos.len() >= 3 && polygon_signed_area_2d(&pos).abs() > 1e-6 {
                    next_polys.push(pos);
                }
            }
        }
        current_polys = next_polys;
    }

    // Deduplicate vertices across all output sub-faces
    let mut unique_verts: Vec<Point2D> = Vec::new();
    let mut out_faces: Vec<Vec<u32>> = Vec::new();

    let mut get_or_insert_vert = |pt: Point2D| -> u32 {
        for (idx, &v) in unique_verts.iter().enumerate() {
            if (v.x - pt.x).abs() < 1e-4 && (v.y - pt.y).abs() < 1e-4 {
                return idx as u32;
            }
        }
        let idx = unique_verts.len() as u32;
        unique_verts.push(pt);
        idx
    };

    for poly in current_polys {
        if poly.len() < 3 {
            continue;
        }
        let poly_indices: Vec<u32> = poly.iter().map(|&pt| get_or_insert_vert(pt)).collect();
        if poly_indices.len() == 4 {
            out_faces.push(poly_indices);
        } else if poly_indices.len() == 3 {
            out_faces.push(poly_indices);
        } else if poly_indices.len() == 5 {
            out_faces.push(vec![poly_indices[0], poly_indices[1], poly_indices[2], poly_indices[3]]);
            out_faces.push(vec![poly_indices[0], poly_indices[3], poly_indices[4]]);
        } else {
            for i in 1..(poly_indices.len() - 1) {
                out_faces.push(vec![poly_indices[0], poly_indices[i], poly_indices[i + 1]]);
            }
        }
    }

    // Compute (s, t) parametric coords and 3D positions for each unique vertex
    let mut out_positions = Vec::with_capacity(unique_verts.len());
    let mut out_uvs = Vec::with_capacity(unique_verts.len());
    let mut out_params = Vec::with_capacity(unique_verts.len());

    let is_quad = positions.len() == 4 && uvs.len() == 4;

    for pt in unique_verts {
        let u = pt.x / tex_w as f32;
        let v = pt.y / tex_h as f32;
        out_uvs.push([u, v]);

        let st = if is_quad {
            invert_quad_bilinear(uvs[0], uvs[1], uvs[2], uvs[3], u, v)
        } else {
            [0.0, 0.0]
        };
        out_params.push(st);

        let pos = if is_quad {
            interpolate_bilinear_3d(positions[0], positions[1], positions[2], positions[3], st[0], st[1])
        } else {
            positions[0]
        };
        out_positions.push(pos);
    }

    // Normal consistency check: ensure all sub-faces point in the same 3D normal direction as original face
    if positions.len() >= 3 && !out_faces.is_empty() {
        let p0 = glam::Vec3::from(positions[0]);
        let p1 = glam::Vec3::from(positions[1]);
        let p2 = glam::Vec3::from(positions[2]);
        let orig_normal = (p1 - p0).cross(p2 - p0);

        for face in &mut out_faces {
            if face.len() >= 3 {
                let sp0 = glam::Vec3::from(out_positions[face[0] as usize]);
                let sp1 = glam::Vec3::from(out_positions[face[1] as usize]);
                let sp2 = glam::Vec3::from(out_positions[face[2] as usize]);
                let sub_normal = (sp1 - sp0).cross(sp2 - sp0);
                if sub_normal.dot(orig_normal) < 0.0 {
                    face.reverse();
                }
            }
        }
    }

    SlicedFaceResult {
        positions: out_positions,
        uvs: out_uvs,
        faces: out_faces,
        param_coords: out_params,
    }
}

/// Performs adaptive pixel grid subdivision on quad faces in a `MeshData` buffer.
///
/// For each face, snaps interior cuts directly to integer pixel grid lines.
pub fn adaptive_pixel_split_mesh(
    mesh: &MeshData,
    face_resolutions: &[Option<(u32, u32)>],
    default_resolution: (u32, u32),
    pixels_per_face: f32,
    max_subdivisions: u32,
    weld_dist: f32,
) -> MeshData {
    let face_count = mesh.indices.len() / 6;
    if face_count == 0 {
        return mesh.clone();
    }

    let mut out = MeshData::new();
    if mesh.secondary_uvs.is_some() {
        out.secondary_uvs = Some(Vec::new());
    }
    if mesh.colors.is_some() {
        out.colors = Some(Vec::new());
    }

    // Clone custom attribute headers
    for (name, attr) in &mesh.custom_attributes {
        let empty_data = match &attr.data {
            AttributeData::Float(_) => AttributeData::Float(Vec::new()),
            AttributeData::Float2(_) => AttributeData::Float2(Vec::new()),
            AttributeData::Float3(_) => AttributeData::Float3(Vec::new()),
            AttributeData::Float4(_) => AttributeData::Float4(Vec::new()),
            AttributeData::Int8(_) => AttributeData::Int8(Vec::new()),
            AttributeData::Int16(_) => AttributeData::Int16(Vec::new()),
            AttributeData::Int32(_) => AttributeData::Int32(Vec::new()),
            AttributeData::UInt8(_) => AttributeData::UInt8(Vec::new()),
            AttributeData::UInt16(_) => AttributeData::UInt16(Vec::new()),
            AttributeData::UInt32(_) => AttributeData::UInt32(Vec::new()),
            AttributeData::String(_) => AttributeData::String(Vec::new()),
            AttributeData::Bool(_) => AttributeData::Bool(Vec::new()),
        };
        out.add_custom_attribute(MeshAttribute {
            name: name.clone(),
            domain: attr.domain,
            data: empty_data,
        });
    }

    for face_idx in 0..face_count {
        let base_tri = face_idx * 6;
        let idx0 = mesh.indices[base_tri] as usize;
        let idx1 = mesh.indices[base_tri + 1] as usize;
        let idx2 = mesh.indices[base_tri + 2] as usize;
        // The 4th vertex of the quad is in the second triangle (idx0, idx2, idx3)
        let idx3 = mesh.indices[base_tri + 5] as usize;

        let mat_id = mesh.face_materials.get(face_idx).copied().unwrap_or(0);
        let tint_idx = mesh.face_tint_indices.get(face_idx).copied().unwrap_or(-1);

        let p0 = mesh.positions[idx0];
        let p1 = mesh.positions[idx1];
        let p2 = mesh.positions[idx2];
        let p3 = mesh.positions[idx3];

        let n0 = mesh.normals.get(idx0).copied().unwrap_or([0.0, 0.0, 1.0]);
        let n1 = mesh.normals.get(idx1).copied().unwrap_or([0.0, 0.0, 1.0]);
        let n2 = mesh.normals.get(idx2).copied().unwrap_or([0.0, 0.0, 1.0]);
        let n3 = mesh.normals.get(idx3).copied().unwrap_or([0.0, 0.0, 1.0]);

        let uv0 = mesh.uvs.get(idx0).copied().unwrap_or([0.0, 0.0]);
        let uv1 = mesh.uvs.get(idx1).copied().unwrap_or([1.0, 0.0]);
        let uv2 = mesh.uvs.get(idx2).copied().unwrap_or([1.0, 1.0]);
        let uv3 = mesh.uvs.get(idx3).copied().unwrap_or([0.0, 1.0]);

        let (tex_w, tex_h) = face_resolutions
            .get(face_idx)
            .and_then(|&res| res)
            .unwrap_or(default_resolution);

        let (u_factors, v_factors) = if pixels_per_face <= 1.0 {
            calculate_pixel_grid_cut_factors(&[uv0, uv1, uv2, uv3], tex_w, tex_h, pixels_per_face, max_subdivisions)
        } else {
            let (c_count, r_count) = calculate_face_target_grid(
                &[uv0, uv1, uv2, uv3],
                tex_w,
                tex_h,
                pixels_per_face,
                max_subdivisions,
            );
            let u_f = (0..=c_count).map(|c| c as f32 / c_count as f32).collect();
            let v_f = (0..=r_count).map(|r| r as f32 / r_count as f32).collect();
            (u_f, v_f)
        };

        let cols = u_factors.len() - 1;
        let rows = v_factors.len() - 1;

        if cols <= 1 && rows <= 1 {
            // No subdivision needed, direct copy quad
            let base_v = out.positions.len() as u32;
            out.positions.extend_from_slice(&[p0, p1, p2, p3]);
            out.normals.extend_from_slice(&[n0, n1, n2, n3]);
            out.uvs.extend_from_slice(&[uv0, uv1, uv2, uv3]);

            if let (Some(sec_in), Some(sec_out)) = (&mesh.secondary_uvs, &mut out.secondary_uvs) {
                sec_out.push(sec_in.get(idx0).copied().unwrap_or([0.0, 0.0]));
                sec_out.push(sec_in.get(idx1).copied().unwrap_or([1.0, 0.0]));
                sec_out.push(sec_in.get(idx2).copied().unwrap_or([1.0, 1.0]));
                sec_out.push(sec_in.get(idx3).copied().unwrap_or([0.0, 1.0]));
            }

            if let (Some(col_in), Some(col_out)) = (&mesh.colors, &mut out.colors) {
                col_out.push(col_in.get(idx0).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]));
                col_out.push(col_in.get(idx1).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]));
                col_out.push(col_in.get(idx2).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]));
                col_out.push(col_in.get(idx3).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]));
            }

            out.indices.extend_from_slice(&[
                base_v, base_v + 1, base_v + 2,
                base_v, base_v + 2, base_v + 3,
            ]);
            out.face_materials.push(mat_id);
            out.face_tint_indices.push(tint_idx);
            continue;
        }

        // Subdivide into (cols + 1) * (rows + 1) grid vertices
        let mut grid_indices: Vec<Vec<u32>> = vec![vec![0; cols + 1]; rows + 1];

        for r in 0..=rows {
            let v_factor = v_factors[r];
            for c in 0..=cols {
                let u_factor = u_factors[c];
                let vert_idx = out.positions.len() as u32;
                grid_indices[r][c] = vert_idx;

                let pos = interpolate_bilinear_3d(p0, p1, p2, p3, u_factor, v_factor);
                let norm = interpolate_bilinear_3d(n0, n1, n2, n3, u_factor, v_factor);
                let uv = interpolate_bilinear_2d(uv0, uv1, uv2, uv3, u_factor, v_factor);

                out.positions.push(pos);
                out.normals.push(norm);
                out.uvs.push(uv);

                if let (Some(sec_in), Some(sec_out)) = (&mesh.secondary_uvs, &mut out.secondary_uvs) {
                    let s0 = sec_in.get(idx0).copied().unwrap_or([0.0, 0.0]);
                    let s1 = sec_in.get(idx1).copied().unwrap_or([1.0, 0.0]);
                    let s2 = sec_in.get(idx2).copied().unwrap_or([1.0, 1.0]);
                    let s3 = sec_in.get(idx3).copied().unwrap_or([0.0, 1.0]);
                    sec_out.push(interpolate_bilinear_2d(s0, s1, s2, s3, u_factor, v_factor));
                }

                if let (Some(col_in), Some(col_out)) = (&mesh.colors, &mut out.colors) {
                    let c0 = col_in.get(idx0).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    let c1 = col_in.get(idx1).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    let c2 = col_in.get(idx2).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    let c3 = col_in.get(idx3).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]);
                    col_out.push(interpolate_bilinear_4d(c0, c1, c2, c3, u_factor, v_factor));
                }
            }
        }

        // Generate sub-quads
        for r in 0..rows {
            for c in 0..cols {
                let v00 = grid_indices[r][c];
                let v10 = grid_indices[r][c + 1];
                let v11 = grid_indices[r + 1][c + 1];
                let v01 = grid_indices[r + 1][c];

                out.indices.extend_from_slice(&[v00, v10, v11, v00, v11, v01]);
                out.face_materials.push(mat_id);
                out.face_tint_indices.push(tint_idx);
            }
        }
    }

    if weld_dist > 0.0 {
        weld_mesh_vertices(&mut out, weld_dist);
    }

    out
}

/// Simple spatial vertex welding to remove duplicate boundary vertices within `weld_dist`.
pub fn weld_mesh_vertices(mesh: &mut MeshData, weld_dist: f32) {
    if mesh.positions.is_empty() || weld_dist <= 0.0 {
        return;
    }

    let inv_dist = 1.0 / weld_dist;
    let mut grid_map: std::collections::HashMap<[i32; 3], u32> = std::collections::HashMap::new();
    let mut remap: Vec<u32> = Vec::with_capacity(mesh.positions.len());
    let mut new_positions: Vec<[f32; 3]> = Vec::new();
    let mut new_normals: Vec<[f32; 3]> = Vec::new();
    let mut new_uvs: Vec<[f32; 2]> = Vec::new();
    let mut new_sec_uvs: Option<Vec<[f32; 2]>> = mesh.secondary_uvs.as_ref().map(|_| Vec::new());
    let mut new_colors: Option<Vec<[f32; 4]>> = mesh.colors.as_ref().map(|_| Vec::new());

    for i in 0..mesh.positions.len() {
        let p = mesh.positions[i];
        let key = [
            (p[0] * inv_dist).round() as i32,
            (p[1] * inv_dist).round() as i32,
            (p[2] * inv_dist).round() as i32,
        ];

        if let Some(&existing_idx) = grid_map.get(&key) {
            // Check if UVs also match closely to avoid welding across UV seams
            let existing_uv = new_uvs[existing_idx as usize];
            let cur_uv = mesh.uvs[i];
            let uv_diff = (existing_uv[0] - cur_uv[0]).abs().max((existing_uv[1] - cur_uv[1]).abs());
            if uv_diff < 1e-4 {
                remap.push(existing_idx);
                continue;
            }
        }

        let new_idx = new_positions.len() as u32;
        grid_map.insert(key, new_idx);
        remap.push(new_idx);

        new_positions.push(p);
        new_normals.push(mesh.normals.get(i).copied().unwrap_or([0.0, 0.0, 1.0]));
        new_uvs.push(mesh.uvs[i]);
        if let (Some(sec_in), Some(sec_out)) = (&mesh.secondary_uvs, &mut new_sec_uvs) {
            sec_out.push(sec_in.get(i).copied().unwrap_or([0.0, 0.0]));
        }
        if let (Some(col_in), Some(col_out)) = (&mesh.colors, &mut new_colors) {
            col_out.push(col_in.get(i).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]));
        }
    }

    // Remap indices
    for idx in &mut mesh.indices {
        *idx = remap[*idx as usize];
    }

    mesh.positions = new_positions;
    mesh.normals = new_normals;
    mesh.uvs = new_uvs;
    mesh.secondary_uvs = new_sec_uvs;
    mesh.colors = new_colors;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_face_target_grid_animated_strip() {
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        // 16x512 animated water strip
        let (cols, rows) = calculate_face_target_grid(&uvs, 16, 512, 1.0, 64);
        assert_eq!(cols, 16);
        assert_eq!(rows, 16, "Must clamp to single square frame 16x16 instead of 512!");
    }

    #[test]
    fn test_calculate_face_target_grid_atlas_subregion() {
        // 16x16 tile inside a 512x512 atlas
        let u0 = 32.0 / 512.0;
        let u1 = 48.0 / 512.0;
        let v0 = 64.0 / 512.0;
        let v1 = 80.0 / 512.0;
        let uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        let (cols, rows) = calculate_face_target_grid(&uvs, 512, 512, 1.0, 64);
        assert_eq!(cols, 16);
        assert_eq!(rows, 16);
    }

    #[test]
    fn test_calculate_face_target_grid_rotated_uv() {
        // 16x16 tile rotated 90 degrees in a 512x512 atlas
        let u0 = 32.0 / 512.0;
        let u1 = 48.0 / 512.0;
        let v0 = 64.0 / 512.0;
        let v1 = 80.0 / 512.0;
        let uvs = [[u1, v0], [u1, v1], [u0, v1], [u0, v0]];
        let (cols, rows) = calculate_face_target_grid(&uvs, 512, 512, 1.0, 64);
        assert_eq!(cols, 16);
        assert_eq!(rows, 16);
    }

    #[test]
    fn test_calculate_pixel_grid_cut_factors_offset_uv() {
        // Quad from (1.3, 2.4) to (4.7, 5.8) in 16x16 pixel space
        let u0 = 1.3 / 16.0;
        let u1 = 4.7 / 16.0;
        let v0 = 2.4 / 16.0;
        let v1 = 5.8 / 16.0;
        let uvs = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];

        let (u_cuts, v_cuts) = calculate_pixel_grid_cut_factors(&uvs, 16, 16, 1.0, 64);
        assert_eq!(u_cuts.len(), 5, "Must have [0.0, cut(2.0), cut(3.0), cut(4.0), 1.0]");
        assert_eq!(v_cuts.len(), 5, "Must have [0.0, cut(3.0), cut(4.0), cut(5.0), 1.0]");

        // Verify that u_cuts[1] corresponds exactly to pixel 2.0
        let px_u1 = (u0 + u_cuts[1] * (u1 - u0)) * 16.0;
        assert!((px_u1 - 2.0).abs() < 1e-4, "Cut 1 must be at integer pixel 2.0, got {}", px_u1);
        let px_u2 = (u0 + u_cuts[2] * (u1 - u0)) * 16.0;
        assert!((px_u2 - 3.0).abs() < 1e-4, "Cut 2 must be at integer pixel 3.0, got {}", px_u2);
        let px_u3 = (u0 + u_cuts[3] * (u1 - u0)) * 16.0;
        assert!((px_u3 - 4.0).abs() < 1e-4, "Cut 3 must be at integer pixel 4.0, got {}", px_u3);
    }

    #[test]
    fn test_calculate_pixel_grid_cut_factors_rotated_90() {
        // 90-degree rotated quad from (2.4, 4.7) to (2.4, 1.3)
        let u0 = 2.4 / 16.0;
        let u1 = 5.8 / 16.0;
        let v0 = 4.7 / 16.0;
        let v1 = 1.3 / 16.0;
        let uvs = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]];

        let (u_cuts, v_cuts) = calculate_pixel_grid_cut_factors(&uvs, 16, 16, 1.0, 64);
        assert_eq!(u_cuts.len(), 5);
        assert_eq!(v_cuts.len(), 5);
    }

    #[test]
    fn test_adaptive_pixel_split_quad() {
        let mut mesh = MeshData::new();
        mesh.positions = vec![
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ];
        mesh.normals = vec![[0.0, 0.0, 1.0]; 4];
        mesh.uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        mesh.face_materials = vec![0];
        mesh.face_tint_indices = vec![-1];

        let res = adaptive_pixel_split_mesh(&mesh, &[Some((2, 2))], (16, 16), 1.0, 64, 0.0);
        assert_eq!(res.indices.len(), 4 * 6, "2x2 sub-quads must result in 4 quads = 24 indices");
        assert_eq!(res.face_materials.len(), 4);
    }

    #[test]
    fn test_slice_face_by_pixel_grid_rotated_45() {
        // Quad on a 3D unit cube face (Z = 0)
        let positions = [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        // 45-degree rotated diamond UV in 16x16 space
        let uvs = [
            [0.5, 0.0],
            [1.0, 0.5],
            [0.5, 1.0],
            [0.0, 0.5],
        ];

        let result = slice_face_by_pixel_grid(&positions, &uvs, 16, 16, 1.0, 64);
        assert!(!result.faces.is_empty(), "Must produce sliced faces");
        assert!(result.faces.len() > 10, "16x16 pixel diamond must produce multiple faces");

        // Verify that in UV space, edges of interior quads are axis-aligned (dx=0 or dy=0)
        let mut interior_quad_count = 0;
        for face in &result.faces {
            if face.len() == 4 {
                let u0 = result.uvs[face[0] as usize];
                let u1 = result.uvs[face[1] as usize];
                let du_01 = (u1[0] - u0[0]) * 16.0;
                let dv_01 = (u1[1] - u0[1]) * 16.0;

                // An interior quad has exact 1.0 px width and height along X or Y
                let is_1x1 = (du_01.abs() - 1.0).abs() < 1e-3 || (dv_01.abs() - 1.0).abs() < 1e-3;
                if is_1x1 {
                    interior_quad_count += 1;
                }
            }
        }
        assert!(interior_quad_count > 0, "Must have 1x1 pixel interior quads");
    }
}
