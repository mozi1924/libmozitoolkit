//! # 2D Pixel-Grid Polygon Clipper
//!
//! Slices 2D and 3D quad/polygon faces strictly along 2D texture pixel grid lines.

use alloc::vec;
use alloc::vec::Vec;

use super::grid::{interpolate_bilinear_3d, invert_quad_bilinear};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Point2D {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

pub(crate) fn polygon_signed_area_2d(poly: &[Point2D]) -> f32 {
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

pub(crate) fn clean_polygon_2d(poly: &[Point2D]) -> Vec<Point2D> {
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

pub(crate) fn clip_polygon_halfplane_2d(
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
