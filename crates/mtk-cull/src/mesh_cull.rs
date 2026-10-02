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

/// Copies a single element from src AttributeData to dst AttributeData at index.
fn copy_attribute_element(src: &AttributeData, dst: &mut AttributeData, idx: usize) {
    match (src, dst) {
        (AttributeData::Float(s), AttributeData::Float(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::Float2(s), AttributeData::Float2(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::Float3(s), AttributeData::Float3(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::Float4(s), AttributeData::Float4(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::Int8(s), AttributeData::Int8(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::Int16(s), AttributeData::Int16(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::Int32(s), AttributeData::Int32(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::UInt8(s), AttributeData::UInt8(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::UInt16(s), AttributeData::UInt16(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::UInt32(s), AttributeData::UInt32(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        (AttributeData::String(s), AttributeData::String(d)) => {
            if let Some(v) = s.get(idx) {
                d.push(v.clone());
            }
        }
        (AttributeData::Bool(s), AttributeData::Bool(d)) => {
            if let Some(&v) = s.get(idx) {
                d.push(v);
            }
        }
        _ => {}
    }
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
    let inv_tol = 1.0 / tol;

    // Canonical Plane Map: [quantized_nx, quantized_ny, quantized_nz, quantized_d] -> list of faces
    let mut plane_buckets: HashMap<[i32; 4], Vec<(usize, glam::Vec3, glam::Vec3, Vec<glam::Vec3>)>> = HashMap::new();
    let mut faces_to_cull: BTreeSet<usize> = BTreeSet::new();

    for face_idx in 0..face_count {
        let base = face_idx * poly_step;
        let mut center = glam::Vec3::ZERO;
        let mut normal = glam::Vec3::ZERO;

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

        let mut face_verts = Vec::with_capacity(num_verts);
        for &vi in &vert_indices {
            let p_raw = mesh.positions.get(vi).copied().unwrap_or([0.0; 3]);
            let p = glam::Vec3::new(p_raw[0], p_raw[1], p_raw[2]);
            face_verts.push(p);
            center += p;

            let n_raw = mesh.normals.get(vi).copied().unwrap_or([0.0, 0.0, 1.0]);
            normal += glam::Vec3::new(n_raw[0], n_raw[1], n_raw[2]);
        }

        let inv_n = 1.0 / (num_verts as f32);
        center *= inv_n;
        normal *= inv_n;
        let norm_len = normal.length();
        if norm_len > 1e-5 {
            normal /= norm_len;
        } else if face_verts.len() >= 3 {
            let e1 = face_verts[1] - face_verts[0];
            let e2 = face_verts[2] - face_verts[0];
            normal = e1.cross(e2).normalize_or_zero();
            if normal.length_squared() < 1e-5 {
                normal = glam::Vec3::Z;
            }
        } else {
            normal = glam::Vec3::Z;
        }

        let d = -normal.dot(center);

        // Canonical plane: enforce normal has non-negative leading component
        let (c_norm, c_d) = if normal.x < -1e-4
            || (normal.x.abs() <= 1e-4 && normal.y < -1e-4)
            || (normal.x.abs() <= 1e-4 && normal.y.abs() <= 1e-4 && normal.z < -1e-4)
        {
            (-normal, -d)
        } else {
            (normal, d)
        };

        let plane_key = [
            (c_norm.x * 10.0).round() as i32,
            (c_norm.y * 10.0).round() as i32,
            (c_norm.z * 10.0).round() as i32,
            (c_d * inv_tol).round() as i32,
        ];

        'search: for delta_d in -1..=1 {
            let search_key = [plane_key[0], plane_key[1], plane_key[2], plane_key[3] + delta_d];
            if let Some(neighbors) = plane_buckets.get(&search_key) {
                for (other_idx, other_norm, _other_center, other_verts) in neighbors {
                    if faces_to_cull.contains(other_idx) {
                        continue;
                    }

                    if let Some(rel) = crate::geometry::coplanar::check_coplanar_overlap(
                        &face_verts,
                        normal,
                        other_verts,
                        *other_norm,
                        tol,
                    ) {
                        let is_same_dir = rel.alignment == crate::geometry::coplanar::FaceAlignment::SameDirection;
                        let is_opp_dir = rel.alignment == crate::geometry::coplanar::FaceAlignment::OppositeDirection;

                        match rel.overlap {
                            crate::geometry::coplanar::CoplanarOverlap::Exact => {
                                if config.cull_duplicates && is_same_dir {
                                    faces_to_cull.insert(face_idx);
                                    break 'search;
                                } else if config.cull_coplanar_opposite && is_opp_dir {
                                    faces_to_cull.insert(face_idx);
                                    faces_to_cull.insert(*other_idx);
                                    break 'search;
                                }
                            }
                            crate::geometry::coplanar::CoplanarOverlap::ContainedInB => {
                                // Face A is completely covered by other face B
                                if (config.cull_duplicates && is_same_dir)
                                    || (config.cull_coplanar_opposite && is_opp_dir)
                                {
                                    faces_to_cull.insert(face_idx);
                                    break 'search;
                                }
                            }
                            crate::geometry::coplanar::CoplanarOverlap::ContainedInA => {
                                // Other face B is completely covered by Face A
                                if (config.cull_duplicates && is_same_dir)
                                    || (config.cull_coplanar_opposite && is_opp_dir)
                                {
                                    faces_to_cull.insert(*other_idx);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        plane_buckets
            .entry(plane_key)
            .or_default()
            .push((face_idx, normal, center, face_verts));
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

    // Copy Mesh-domain attributes directly
    for (name, attr) in &mesh.custom_attributes {
        if attr.domain == mtk_core::attributes::AttributeDomain::Mesh {
            if let Some(out_attr) = out.custom_attributes.get_mut(name) {
                out_attr.data = attr.data.clone();
            }
        }
    }

    let mut remaining_faces = 0;
    let mut out_quad_indices = if is_quad_mesh || mesh.quad_indices.is_some() {
        Some(Vec::new())
    } else {
        None
    };

    for face_idx in 0..face_count {
        if faces_to_cull.contains(&face_idx) {
            continue;
        }

        remaining_faces += 1;
        let base = face_idx * poly_step;
        let quad_start_v = out.positions.len() as u32;

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

            // Copy Point and Corner domain attributes
            for (name, in_attr) in &mesh.custom_attributes {
                if in_attr.domain == mtk_core::attributes::AttributeDomain::Point {
                    if let Some(out_attr) = out.custom_attributes.get_mut(name) {
                        copy_attribute_element(&in_attr.data, &mut out_attr.data, orig_idx);
                    }
                } else if in_attr.domain == mtk_core::attributes::AttributeDomain::Corner {
                    if let Some(out_attr) = out.custom_attributes.get_mut(name) {
                        copy_attribute_element(&in_attr.data, &mut out_attr.data, base + step);
                    }
                }
            }

            out.indices.push(new_v_idx);
        }

        // Quad indices maintenance: 4 corner vertex indices
        if is_quad_mesh {
            if let Some(ref mut q_indices) = out_quad_indices {
                q_indices.push(quad_start_v);
                q_indices.push(quad_start_v + 1);
                q_indices.push(quad_start_v + 2);
                q_indices.push(quad_start_v + 5);
            }
        }

        if let Some(&mat_id) = mesh.face_materials.get(face_idx) {
            out.face_materials.push(mat_id);
        }
        if let Some(&tint_idx) = mesh.face_tint_indices.get(face_idx) {
            out.face_tint_indices.push(tint_idx);
        }

        // Copy Face domain attributes
        for (name, in_attr) in &mesh.custom_attributes {
            if in_attr.domain == mtk_core::attributes::AttributeDomain::Face {
                if let Some(out_attr) = out.custom_attributes.get_mut(name) {
                    copy_attribute_element(&in_attr.data, &mut out_attr.data, face_idx);
                }
            }
        }
    }

    out.quad_indices = out_quad_indices;

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

    #[test]
    fn test_cull_duplicate_faces_preserving_attributes() {
        let mut mesh = MeshData::new();
        // Quad 1: Facing +Z
        mesh.positions.extend_from_slice(&[
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]);
        mesh.normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);
        mesh.uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        mesh.face_materials.push(10);
        mesh.face_tint_indices.push(-1);

        // Quad 2: Identical duplicate face at same position facing +Z
        mesh.positions.extend_from_slice(&[
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]);
        mesh.normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);
        mesh.uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        mesh.indices.extend_from_slice(&[4, 5, 6, 4, 6, 7]);
        mesh.face_materials.push(20);
        mesh.face_tint_indices.push(-1);

        mesh.add_custom_attribute(MeshAttribute::new(
            "test_attr",
            mtk_core::attributes::AttributeDomain::Face,
            AttributeData::String(vec!["face_0".to_string(), "face_1".to_string()]),
        ));

        let res = cull_mesh_faces(&mesh, &MeshCullConfig::default());
        assert_eq!(res.initial_faces, 2);
        assert_eq!(res.culled_faces, 1, "Duplicate face must be culled to eliminate Z-fighting");
        assert_eq!(res.remaining_faces, 1);
        assert_eq!(res.mesh.face_materials, vec![10]);

        // Verify custom attributes preserved
        let attr = res.mesh.custom_attributes.get("test_attr").expect("test_attr must exist");
        if let AttributeData::String(ref vals) = attr.data {
            assert_eq!(vals.len(), 1);
            assert_eq!(vals[0], "face_0");
        } else {
            panic!("Expected String attribute data");
        }
    }
}
