use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::attributes::{
    AttributeData, FaceAttributes, MaterialSlotId, MeshAttribute, TintIndex,
};
use crate::geometry::Quad;

/// Flat, contiguous mesh data buffer designed for zero-copy or direct buffer transfers
/// into graphics APIs (OpenGL/WebGPU/Vulkan) or 3D host engines (Blender, Bevy, Three.js).
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MeshData {
    /// Contiguous 3D vertex positions `[x, y, z]`.
    pub positions: Vec<[f32; 3]>,
    /// Contiguous vertex normal vectors `[nx, ny, nz]`.
    pub normals: Vec<[f32; 3]>,
    /// Triangle face indices (every 3 consecutive u32 form a triangle).
    pub indices: Vec<u32>,
    /// Primary UV coordinates `[u, v]`.
    pub uvs: Vec<[f32; 2]>,
    /// Secondary UV coordinates (e.g. for atlas tiling or colormap sampling).
    pub secondary_uvs: Option<Vec<[f32; 2]>>,
    /// Vertex RGBA colors `[r, g, b, a]` in linear color space.
    pub colors: Option<Vec<[f32; 4]>>,
    /// Material slot ID per face (per 2 triangles in quad baking).
    pub face_materials: Vec<MaterialSlotId>,
    /// Tint index per face.
    pub face_tint_indices: Vec<TintIndex>,
    /// Generic typed custom attributes indexed by attribute name (Domain: Point/Corner/Face/Mesh).
    pub custom_attributes: HashMap<String, MeshAttribute>,
}

impl MeshData {
    /// Creates a new empty `MeshData` buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new `MeshData` with pre-allocated capacity for efficiency.
    pub fn with_capacity(num_vertices: usize, num_indices: usize, num_faces: usize) -> Self {
        Self {
            positions: Vec::with_capacity(num_vertices),
            normals: Vec::with_capacity(num_vertices),
            indices: Vec::with_capacity(num_indices),
            uvs: Vec::with_capacity(num_vertices),
            secondary_uvs: None,
            colors: None,
            face_materials: Vec::with_capacity(num_faces),
            face_tint_indices: Vec::with_capacity(num_faces),
            custom_attributes: HashMap::new(),
        }
    }

    /// Number of vertices in the mesh buffer.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles in the mesh buffer.
    #[inline]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Number of quad-level faces recorded in `face_materials`.
    #[inline]
    pub fn face_count(&self) -> usize {
        self.face_materials.len()
    }

    /// Checks if the mesh data buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Adds or replaces a custom attribute on the mesh.
    pub fn add_custom_attribute(&mut self, attr: MeshAttribute) {
        self.custom_attributes.insert(attr.name.clone(), attr);
    }

    /// Gets a reference to a custom attribute by name.
    pub fn get_custom_attribute(&self, name: &str) -> Option<&MeshAttribute> {
        self.custom_attributes.get(name)
    }

    /// Gets a mutable reference to a custom attribute by name.
    pub fn get_custom_attribute_mut(&mut self, name: &str) -> Option<&mut MeshAttribute> {
        self.custom_attributes.get_mut(name)
    }

    /// Removes a custom attribute by name, returning it if present.
    pub fn remove_custom_attribute(&mut self, name: &str) -> Option<MeshAttribute> {
        self.custom_attributes.remove(name)
    }

    /// Checks if a custom attribute exists.
    pub fn has_custom_attribute(&self, name: &str) -> bool {
        self.custom_attributes.contains_key(name)
    }

    /// Returns a list of all custom attribute names.
    pub fn custom_attribute_names(&self) -> Vec<String> {
        self.custom_attributes.keys().cloned().collect()
    }

    /// Clears all vertices, indices, and attributes while retaining memory allocations.
    pub fn clear(&mut self) {
        self.positions.clear();
        self.normals.clear();
        self.indices.clear();
        self.uvs.clear();
        if let Some(ref mut sec_uvs) = self.secondary_uvs {
            sec_uvs.clear();
        }
        if let Some(ref mut cols) = self.colors {
            cols.clear();
        }
        self.face_materials.clear();
        self.face_tint_indices.clear();
        self.custom_attributes.clear();
    }

    /// Appends a quad (4 vertices, 2 triangles: 0-1-2 and 0-2-3) and its face attributes.
    pub fn append_quad(&mut self, quad: &Quad, attributes: &FaceAttributes) {
        let base_idx = self.positions.len() as u32;

        let n = [quad.normal.x, quad.normal.y, quad.normal.z];
        for i in 0..4 {
            let v = quad.vertices[i];
            let uv = quad.uvs[i];
            self.positions.push([v.x, v.y, v.z]);
            self.normals.push(n);
            self.uvs.push([uv.x, uv.y]);
        }

        // Triangulate CCW quad: 0-1-2 and 0-2-3
        self.indices.push(base_idx);
        self.indices.push(base_idx + 1);
        self.indices.push(base_idx + 2);

        self.indices.push(base_idx);
        self.indices.push(base_idx + 2);
        self.indices.push(base_idx + 3);

        self.face_materials.push(attributes.material_slot);
        self.face_tint_indices.push(attributes.tint_index);
    }

    /// Appends another `MeshData` into this mesh data buffer.
    pub fn append_mesh(&mut self, other: &MeshData) {
        let base_idx = self.positions.len() as u32;

        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);

        self.indices.reserve(other.indices.len());
        for &idx in &other.indices {
            self.indices.push(base_idx + idx);
        }

        self.face_materials.extend_from_slice(&other.face_materials);
        self.face_tint_indices.extend_from_slice(&other.face_tint_indices);

        if let Some(ref other_sec) = other.secondary_uvs {
            let sec = self.secondary_uvs.get_or_insert_with(Vec::new);
            sec.extend_from_slice(other_sec);
        }

        if let Some(ref other_col) = other.colors {
            let col = self.colors.get_or_insert_with(Vec::new);
            col.extend_from_slice(other_col);
        }

        for (name, attr) in &other.custom_attributes {
            if let Some(existing) = self.custom_attributes.get_mut(name) {
                if existing.domain == attr.domain {
                    match (&mut existing.data, &attr.data) {
                        (AttributeData::Float(a), AttributeData::Float(b)) => a.extend_from_slice(b),
                        (AttributeData::Float2(a), AttributeData::Float2(b)) => a.extend_from_slice(b),
                        (AttributeData::Float3(a), AttributeData::Float3(b)) => a.extend_from_slice(b),
                        (AttributeData::Float4(a), AttributeData::Float4(b)) => a.extend_from_slice(b),
                        (AttributeData::Int8(a), AttributeData::Int8(b)) => a.extend_from_slice(b),
                        (AttributeData::Int16(a), AttributeData::Int16(b)) => a.extend_from_slice(b),
                        (AttributeData::Int32(a), AttributeData::Int32(b)) => a.extend_from_slice(b),
                        (AttributeData::UInt8(a), AttributeData::UInt8(b)) => a.extend_from_slice(b),
                        (AttributeData::UInt16(a), AttributeData::UInt16(b)) => a.extend_from_slice(b),
                        (AttributeData::UInt32(a), AttributeData::UInt32(b)) => a.extend_from_slice(b),
                        (AttributeData::Bool(a), AttributeData::Bool(b)) => a.extend_from_slice(b),
                        (AttributeData::String(a), AttributeData::String(b)) => a.extend_from_slice(b),
                        _ => {}
                    }
                }
            } else {
                self.custom_attributes.insert(name.clone(), attr.clone());
            }
        }
    }

    /// Returns a flat contiguous slice of vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    ///
    /// Memory layout is guaranteed continuous `f32` with no padding.
    #[inline]
    pub fn positions_flat(&self) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(
                self.positions.as_ptr() as *const f32,
                self.positions.len() * 3,
            )
        }
    }

    /// Returns a flat contiguous slice of vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    #[inline]
    pub fn normals_flat(&self) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(
                self.normals.as_ptr() as *const f32,
                self.normals.len() * 3,
            )
        }
    }

    /// Returns a flat contiguous slice of primary UVs `[u0, v0, u1, v1, ...]`.
    #[inline]
    pub fn uvs_flat(&self) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(
                self.uvs.as_ptr() as *const f32,
                self.uvs.len() * 2,
            )
        }
    }

    /// Returns a flat contiguous slice of vertex colors `[r0, g0, b0, a0, ...]` if present.
    #[inline]
    pub fn colors_flat(&self) -> Option<&[f32]> {
        self.colors.as_ref().map(|cols| unsafe {
            std::slice::from_raw_parts(
                cols.as_ptr() as *const f32,
                cols.len() * 4,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::AttributeDomain;
    use crate::direction::Direction;

    #[test]
    fn test_mesh_append_quad() {
        let mut mesh = MeshData::new();
        let quad = Quad::unit_cube_face(Direction::Up);
        let attr = FaceAttributes {
            material_slot: 1,
            tint_index: 0,
            ..Default::default()
        };

        mesh.append_quad(&quad, &attr);
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.face_count(), 1);
        assert_eq!(mesh.face_materials[0], 1);
        assert_eq!(mesh.face_tint_indices[0], 0);

        let pos_flat = mesh.positions_flat();
        assert_eq!(pos_flat.len(), 12);
        assert_eq!(pos_flat[0], mesh.positions[0][0]);
        assert_eq!(pos_flat[1], mesh.positions[0][1]);
        assert_eq!(pos_flat[2], mesh.positions[0][2]);

        let norm_flat = mesh.normals_flat();
        assert_eq!(norm_flat.len(), 12);
        assert_eq!(norm_flat[0], 0.0);
        assert_eq!(norm_flat[1], 1.0);
        assert_eq!(norm_flat[2], 0.0);

        let uvs_flat = mesh.uvs_flat();
        assert_eq!(uvs_flat.len(), 8);
    }

    #[test]
    fn test_mesh_custom_attributes() {
        let mut mesh = MeshData::new();
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_atlas_chunk_id",
            AttributeDomain::Face,
            AttributeData::Int32(vec![0, 1, 2]),
        ));
        mesh.add_custom_attribute(MeshAttribute::new(
            "mtk_source_texture_key",
            AttributeDomain::Face,
            AttributeData::String(vec!["minecraft:block/stone".to_string()]),
        ));

        assert!(mesh.has_custom_attribute("mtk_atlas_chunk_id"));
        assert_eq!(mesh.get_custom_attribute("mtk_atlas_chunk_id").unwrap().len(), 3);
        assert_eq!(mesh.get_custom_attribute("mtk_atlas_chunk_id").unwrap().as_bytes().unwrap().len(), 12);
        assert_eq!(mesh.get_custom_attribute("mtk_source_texture_key").unwrap().as_bytes(), None);

        let mut other = MeshData::new();
        other.add_custom_attribute(MeshAttribute::new(
            "mtk_atlas_chunk_id",
            AttributeDomain::Face,
            AttributeData::Int32(vec![3, 4]),
        ));

        mesh.append_mesh(&other);
        let merged_chunk_attr = mesh.get_custom_attribute("mtk_atlas_chunk_id").unwrap();
        assert_eq!(merged_chunk_attr.len(), 5);
        if let AttributeData::Int32(ref vals) = merged_chunk_attr.data {
            assert_eq!(vals, &vec![0, 1, 2, 3, 4]);
        } else {
            panic!("Expected Int32 attribute data");
        }
    }
}
