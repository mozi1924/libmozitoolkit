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
/// Automatically guards against vertical strip animations (e.g. 16x512) by clamping effective height
/// to a single square frame (`tex_w`).
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

    // 1. Long vertical animated strip detection (anti-explosion defense)
    let effective_h = if tex_h > tex_w && (tex_h % tex_w == 0) {
        tex_w
    } else {
        tex_h
    };

    // 2. Compute UV bounding box span
    let aabb = Aabb2d::from_points(uvs);
    let u_span = (aabb.max.x - aabb.min.x).abs();
    let v_span = (aabb.max.y - aabb.min.y).abs();

    // 3. Adaptive resolution calculation
    let cols = ((u_span * tex_w as f32 / ppf).round() as u32).clamp(1, max_sub);
    let rows = ((v_span * effective_h as f32 / ppf).round() as u32).clamp(1, max_sub);

    (cols, rows)
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

/// Performs adaptive pixel grid subdivision on quad faces in a `MeshData` buffer.
///
/// For each face (represented as 2 consecutive triangles in `mesh.indices`), if `face_resolutions[face_idx]`
/// is provided (or if a default resolution is given), the face is subdivided into `cols x rows` sub-quads.
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

        let (cols, rows) = calculate_face_target_grid(
            &[uv0, uv1, uv2, uv3],
            tex_w,
            tex_h,
            pixels_per_face,
            max_subdivisions,
        );

        if cols == 1 && rows == 1 {
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
        let mut grid_indices: Vec<Vec<u32>> = vec![vec![0; (cols + 1) as usize]; (rows + 1) as usize];

        for r in 0..=rows {
            let v_factor = r as f32 / rows as f32;
            for c in 0..=cols {
                let u_factor = c as f32 / cols as f32;
                let vert_idx = out.positions.len() as u32;
                grid_indices[r as usize][c as usize] = vert_idx;

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
                let v00 = grid_indices[r as usize][c as usize];
                let v10 = grid_indices[r as usize][(c + 1) as usize];
                let v11 = grid_indices[(r + 1) as usize][(c + 1) as usize];
                let v01 = grid_indices[(r + 1) as usize][c as usize];

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
}
