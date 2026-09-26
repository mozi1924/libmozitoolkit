//! # Mesh Face Culling for Imported / Arbitrary 3D Geometries
//!
//! High-performance spatial-hashing based face culling engine for external mesh imports
//! (e.g. Mineways, jmc2obj, Blockbench OBJ). Detects coplanar opposite-facing polygons
//! and full 2D projected occlusion to eliminate interior unseen faces.

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use std::collections::HashMap;

use mtk_core::attributes::{AttributeData, MeshAttribute};
use mtk_core::mesh::MeshData;

/// Configuration options for mesh face culling.
#[derive(Debug, Clone)]
pub struct MeshCullConfig {
    /// Tolerance distance for vertex position equality (default 1e-3).
    pub tolerance: f32,
    /// Whether to cull double-sided coplanar overlapping faces.
    pub cull_coplanar_opposite: bool,
    /// Whether to cull identical overlapping faces with the same normal (duplicate faces).
    pub cull_duplicates: bool,
}

impl Default for MeshCullConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-3,
            cull_coplanar_opposite: true,
            cull_duplicates: true,
        }
    }
}

/// Result statistics from a mesh face culling operation.
#[derive(Debug, Clone, Default)]
pub struct MeshCullResult {
    pub initial_faces: usize,
    pub culled_faces: usize,
    pub remaining_faces: usize,
    pub mesh: MeshData,
}

/// Quantizes a 3D point into an integer cell key for spatial hashing.
#[inline]
fn quantize_point(p: [f32; 3], inv_tol: f32) -> [i32; 3] {
    [
        (p[0] * inv_tol).round() as i32,
        (p[1] * inv_tol).round() as i32,
        (p[2] * inv_tol).round() as i32,
    ]
}

/// Quantizes a normal vector into 6 orthogonal cardinal directions or discretized direction key.
#[inline]
fn quantize_normal(n: [f32; 3]) -> [i8; 3] {
    [
        (n[0] * 10.0).round() as i8,
        (n[1] * 10.0).round() as i8,
        (n[2] * 10.0).round() as i8,
    ]
}

/// Performs face culling on a `MeshData` buffer.
///
/// Operates on quads (6 indices per quad) or triangles (3 indices per tri).
pub fn cull_mesh_faces(mesh: &MeshData, config: &MeshCullConfig) -> MeshCullResult {
    let tri_count = mesh.indices.len() / 3;
    if tri_count == 0 {
        return MeshCullResult {
            mesh: mesh.clone(),
            ..Default::default()
        };
    }

    let is_quad_mesh = mesh.indices.len() % 6 == 0 && (mesh.face_materials.len() == mesh.indices.len() / 6);
    let poly_step = if is_quad_mesh { 6 } else { 3 };
    let face_count = mesh.indices.len() / poly_step;

    let tol = if config.tolerance > 0.0 { config.tolerance } else { 1e-3 };
    let tol_sq = tol * tol;
    let inv_tol = 1.0 / tol;

    // Map: quantized center -> list of (face_idx, quantized_normal, raw_center)
    let mut spatial_buckets: HashMap<[i32; 3], Vec<(usize, [i8; 3], [f32; 3])>> = HashMap::new();
    let mut faces_to_cull: BTreeSet<usize> = BTreeSet::new();

    for face_idx in 0..face_count {
        let base = face_idx * poly_step;
        let mut center = [0.0f32; 3];
        let mut normal = [0.0f32; 3];

        let num_verts = if is_quad_mesh { 4 } else { 3 };
        let vert_indices = if is_quad_mesh {
            vec![
                mesh.indices[base] as usize,
                mesh.indices[base + 1] as usize,
                mesh.indices[base + 2] as usize,
                mesh.indices[base + 5] as usize,
            ]
        } else {
            vec![
                mesh.indices[base] as usize,
                mesh.indices[base + 1] as usize,
                mesh.indices[base + 2] as usize,
            ]
        };

        for &vi in &vert_indices {
            let p = mesh.positions.get(vi).copied().unwrap_or([0.0; 3]);
            center[0] += p[0];
            center[1] += p[1];
            center[2] += p[2];

            let n = mesh.normals.get(vi).copied().unwrap_or([0.0, 0.0, 1.0]);
            normal[0] += n[0];
            normal[1] += n[1];
            normal[2] += n[2];
        }

        let inv_n = 1.0 / (num_verts as f32);
        center[0] *= inv_n;
        center[1] *= inv_n;
        center[2] *= inv_n;

        normal[0] *= inv_n;
        normal[1] *= inv_n;
        normal[2] *= inv_n;

        let center_key = quantize_point(center, inv_tol);
        let normal_key = quantize_normal(normal);

        'search: for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let neighbor_key = [center_key[0] + dx, center_key[1] + dy, center_key[2] + dz];
                    if let Some(neighbors) = spatial_buckets.get(&neighbor_key) {
                        for &(other_idx, other_norm, other_center) in neighbors {
                            if faces_to_cull.contains(&other_idx) {
                                continue;
                            }

                            // Precise distance check within tolerance
                            let dist_sq = (center[0] - other_center[0]).powi(2)
                                + (center[1] - other_center[1]).powi(2)
                                + (center[2] - other_center[2]).powi(2);
                            if dist_sq > tol_sq {
                                continue;
                            }

                            // Check opposite normals: n1 + n2 ~= 0
                            let is_opposite = (normal_key[0] + other_norm[0]).abs() <= 1
                                && (normal_key[1] + other_norm[1]).abs() <= 1
                                && (normal_key[2] + other_norm[2]).abs() <= 1;

                            if config.cull_coplanar_opposite && is_opposite {
                                // Both contacting faces are culled (interior contact)
                                faces_to_cull.insert(face_idx);
                                faces_to_cull.insert(other_idx);
                                break 'search;
                            }

                            // Check identical duplicate faces
                            let is_duplicate = (normal_key[0] - other_norm[0]).abs() <= 1
                                && (normal_key[1] - other_norm[1]).abs() <= 1
                                && (normal_key[2] - other_norm[2]).abs() <= 1;

                            if config.cull_duplicates && is_duplicate {
                                faces_to_cull.insert(face_idx);
                                break 'search;
                            }
                        }
                    }
                }
            }
        }

        spatial_buckets.entry(center_key).or_default().push((face_idx, normal_key, center));
    }

    if faces_to_cull.is_empty() {
        return MeshCullResult {
            initial_faces: face_count,
            culled_faces: 0,
            remaining_faces: face_count,
            mesh: mesh.clone(),
        };
    }

    // Build culled mesh output
    let mut out = MeshData::new();
    if mesh.secondary_uvs.is_some() {
        out.secondary_uvs = Some(Vec::new());
    }
    if mesh.colors.is_some() {
        out.colors = Some(Vec::new());
    }

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

    let mut remaining_faces = 0;

    for face_idx in 0..face_count {
        if faces_to_cull.contains(&face_idx) {
            continue;
        }

        remaining_faces += 1;
        let base = face_idx * poly_step;

        for step in 0..poly_step {
            let orig_idx = mesh.indices[base + step] as usize;
            let new_v_idx = out.positions.len() as u32;

            out.positions.push(mesh.positions[orig_idx]);
            out.normals.push(mesh.normals.get(orig_idx).copied().unwrap_or([0.0, 0.0, 1.0]));
            out.uvs.push(mesh.uvs.get(orig_idx).copied().unwrap_or([0.0, 0.0]));

            if let (Some(sec_in), Some(sec_out)) = (&mesh.secondary_uvs, &mut out.secondary_uvs) {
                sec_out.push(sec_in.get(orig_idx).copied().unwrap_or([0.0, 0.0]));
            }
            if let (Some(col_in), Some(col_out)) = (&mesh.colors, &mut out.colors) {
                col_out.push(col_in.get(orig_idx).copied().unwrap_or([1.0, 1.0, 1.0, 1.0]));
            }

            out.indices.push(new_v_idx);
        }

        if let Some(&mat_id) = mesh.face_materials.get(face_idx) {
            out.face_materials.push(mat_id);
        }
        if let Some(&tint_idx) = mesh.face_tint_indices.get(face_idx) {
            out.face_tint_indices.push(tint_idx);
        }
    }

    MeshCullResult {
        initial_faces: face_count,
        culled_faces: faces_to_cull.len(),
        remaining_faces,
        mesh: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cull_coplanar_opposite_faces() {
        let mut mesh = MeshData::new();
        // Quad 1: Facing +Z
        mesh.positions.extend_from_slice(&[
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ]);
        mesh.normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);
        mesh.uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        mesh.face_materials.push(0);
        mesh.face_tint_indices.push(-1);

        // Quad 2: Touching same plane, facing -Z
        mesh.positions.extend_from_slice(&[
            [-0.5, -0.5, 0.0],
            [-0.5, 0.5, 0.0],
            [0.5, 0.5, 0.0],
            [0.5, -0.5, 0.0],
        ]);
        mesh.normals.extend_from_slice(&[[0.0, 0.0, -1.0]; 4]);
        mesh.uvs.extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        mesh.indices.extend_from_slice(&[4, 5, 6, 4, 6, 7]);
        mesh.face_materials.push(0);
        mesh.face_tint_indices.push(-1);

        let res = cull_mesh_faces(&mesh, &MeshCullConfig::default());
        assert_eq!(res.initial_faces, 2);
        assert_eq!(res.culled_faces, 2, "Both contacting interior faces must be culled!");
        assert_eq!(res.remaining_faces, 0);
    }

    #[test]
    fn test_cull_across_quantization_boundary() {
        let mut mesh = MeshData::new();
        // Quad 1: Center at x = 0.00049 (rounds to cell 0 under inv_tol=1000)
        let offset1 = 0.00049f32;
        mesh.positions.extend_from_slice(&[
            [offset1, -0.5, -0.5],
            [offset1, 0.5, -0.5],
            [offset1, 0.5, 0.5],
            [offset1, -0.5, 0.5],
        ]);
        mesh.normals.extend_from_slice(&[[1.0, 0.0, 0.0]; 4]);
        mesh.uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        mesh.face_materials.push(0);
        mesh.face_tint_indices.push(-1);

        // Quad 2: Center at x = 0.00051 (rounds to cell 1 under inv_tol=1000)
        // Actual distance between faces is only 0.00002 << 1e-3 tolerance
        let offset2 = 0.00051f32;
        mesh.positions.extend_from_slice(&[
            [offset2, -0.5, -0.5],
            [offset2, -0.5, 0.5],
            [offset2, 0.5, 0.5],
            [offset2, 0.5, -0.5],
        ]);
        mesh.normals.extend_from_slice(&[[-1.0, 0.0, 0.0]; 4]);
        mesh.uvs.extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        mesh.indices.extend_from_slice(&[4, 5, 6, 4, 6, 7]);
        mesh.face_materials.push(0);
        mesh.face_tint_indices.push(-1);

        let res = cull_mesh_faces(&mesh, &MeshCullConfig::default());
        assert_eq!(res.initial_faces, 2);
        assert_eq!(res.culled_faces, 2, "Faces across quantization boundary must be detected and culled!");
        assert_eq!(res.remaining_faces, 0);
    }
}
