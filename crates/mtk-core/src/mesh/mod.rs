//! # Mesh Data Buffer and Geometry Pipeline
//!
//! Provides flat contiguous POD mesh buffer abstractions (`MeshData`) designed
//! for zero-copy transfers into graphics APIs and DCC host environments.

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::attributes::{FaceAttributes, MaterialSlotId, MeshAttribute, TintIndex};
use crate::geometry::Quad;

pub mod attributes;
pub mod merge;
pub mod weld;

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
    /// Optional Quad polygon vertex indices (4 u32 per quad) preserving exact corner cyclic order.
    pub quad_indices: Option<Vec<u32>>,
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
            quad_indices: None,
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

    /// Number of quad faces recorded.
    #[inline]
    pub fn quad_count(&self) -> usize {
        if let Some(ref quads) = self.quad_indices {
            quads.len() / 4
        } else {
            self.indices.len() / 6
        }
    }

    /// Quad polygon vertex indices `[v0, v1, v2, v3, ...]` (4 u32 per quad).
    ///
    /// If `quad_indices` is present, returns a clone.
    /// Otherwise, reconstructs quad face polygons by topologically pairing adjacent triangle pairs.
    pub fn reconstruct_quad_indices(&self) -> Vec<u32> {
        if let Some(ref quads) = self.quad_indices {
            return quads.clone();
        }
        let quad_count = self.indices.len() / 6;
        let mut quads = Vec::with_capacity(quad_count * 4);
        for q in 0..quad_count {
            let base = q * 6;
            let t1 = [
                self.indices[base],
                self.indices[base + 1],
                self.indices[base + 2],
            ];
            let t2 = [
                self.indices[base + 3],
                self.indices[base + 4],
                self.indices[base + 5],
            ];

            // Identify the unique non-shared vertex of triangle 1 (O1)
            let mut o1_idx = 0;
            for i in 0..3 {
                if !t2.contains(&t1[i]) {
                    o1_idx = i;
                    break;
                }
            }
            let o1 = t1[o1_idx];
            let d1 = t1[(o1_idx + 2) % 3]; // predecessor in t1
            let d2 = t1[(o1_idx + 1) % 3]; // successor in t1

            // Identify the unique non-shared vertex of triangle 2 (O2)
            let o2 = t2.iter().copied().find(|v| !t1.contains(v)).unwrap_or(d2);

            quads.push(d1);
            quads.push(o1);
            quads.push(d2);
            quads.push(o2);
        }
        quads
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
        if let Some(ref mut quads) = self.quad_indices {
            quads.clear();
        }
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

        let quads = self.quad_indices.get_or_insert_with(Vec::new);
        quads.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx + 3]);

        self.face_materials.push(attributes.material_slot);
        self.face_tint_indices.push(attributes.tint_index);
    }

    /// Appends another `MeshData` into this mesh data buffer.
    pub fn append_mesh(&mut self, other: &MeshData) {
        let base_idx = self.positions.len() as u32;

        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);

        let old_idx_len = self.indices.len();
        self.indices.extend_from_slice(&other.indices);
        for idx in &mut self.indices[old_idx_len..] {
            *idx += base_idx;
        }

        if let Some(ref o_quads) = other.quad_indices {
            let quads = self.quad_indices.get_or_insert_with(Vec::new);
            let old_quad_len = quads.len();
            quads.extend_from_slice(o_quads);
            for idx in &mut quads[old_quad_len..] {
                *idx += base_idx;
            }
        }

        self.face_materials.extend_from_slice(&other.face_materials);
        self.face_tint_indices
            .extend_from_slice(&other.face_tint_indices);

        if let Some(ref other_sec) = other.secondary_uvs {
            let sec = self.secondary_uvs.get_or_insert_with(Vec::new);
            sec.extend_from_slice(other_sec);
        }

        if let Some(ref other_col) = other.colors {
            let col = self.colors.get_or_insert_with(Vec::new);
            col.extend_from_slice(other_col);
        }

        Self::extend_custom_attributes(&mut self.custom_attributes, &other.custom_attributes);
    }

    /// Returns a flat contiguous slice of vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    #[inline]
    pub fn positions_flat(&self) -> &[f32] {
        bytemuck::cast_slice(&self.positions)
    }

    /// Returns a flat contiguous slice of vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    #[inline]
    pub fn normals_flat(&self) -> &[f32] {
        bytemuck::cast_slice(&self.normals)
    }

    /// Returns a flat contiguous slice of primary UVs `[u0, v0, u1, v1, ...]`.
    #[inline]
    pub fn uvs_flat(&self) -> &[f32] {
        bytemuck::cast_slice(&self.uvs)
    }

    /// Returns a flat contiguous slice of vertex colors `[r0, g0, b0, a0, ...]` if present.
    #[inline]
    pub fn colors_flat(&self) -> Option<&[f32]> {
        self.colors.as_ref().map(|cols| bytemuck::cast_slice(cols))
    }

    /// Returns a sorted list of unique material slot IDs present across all faces in this mesh.
    pub fn used_materials(&self) -> Vec<MaterialSlotId> {
        let mut set = std::collections::BTreeSet::new();
        for &m in &self.face_materials {
            set.insert(m);
        }
        set.into_iter().collect()
    }

    /// Compacts `face_materials` to contiguous indices `0..N-1` and returns the mapping of original material IDs.
    pub fn compact_materials(&mut self) -> Vec<MaterialSlotId> {
        let original_ids = self.used_materials();
        if original_ids.is_empty() {
            return Vec::new();
        }
        let id_to_slot: HashMap<MaterialSlotId, MaterialSlotId> = original_ids
            .iter()
            .enumerate()
            .map(|(slot, &orig)| (orig, slot as MaterialSlotId))
            .collect();
        for m in &mut self.face_materials {
            if let Some(&new_slot) = id_to_slot.get(m) {
                *m = new_slot;
            }
        }
        original_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{AttributeData, AttributeDomain};
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
        assert_eq!(
            mesh.get_custom_attribute("mtk_atlas_chunk_id")
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            mesh.get_custom_attribute("mtk_atlas_chunk_id")
                .unwrap()
                .as_bytes()
                .unwrap()
                .len(),
            12
        );
        assert_eq!(
            mesh.get_custom_attribute("mtk_source_texture_key")
                .unwrap()
                .as_bytes(),
            None
        );

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

    #[test]
    fn test_mesh_used_and_compact_materials() {
        let mut mesh = MeshData::new();
        mesh.face_materials = vec![5, 0, 5, 12, 0];
        assert_eq!(mesh.used_materials(), vec![0, 5, 12]);

        let original_mapping = mesh.compact_materials();
        assert_eq!(original_mapping, vec![0, 5, 12]);
        assert_eq!(mesh.face_materials, vec![1, 0, 1, 2, 0]);
        assert_eq!(mesh.used_materials(), vec![0, 1, 2]);
    }

    #[test]
    fn test_mesh_flat_accessors() {
        let mut mesh = MeshData::new();
        mesh.positions = vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]];
        mesh.normals = vec![[0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        mesh.uvs = vec![[0.1, 0.2], [0.3, 0.4]];
        mesh.colors = Some(vec![[1.0, 0.0, 0.0, 1.0]]);

        assert_eq!(mesh.positions_flat(), &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(mesh.normals_flat(), &[0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(mesh.uvs_flat(), &[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(mesh.colors_flat().unwrap(), &[1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_mesh_merge_welded_sections() {
        let mut m1 = MeshData::new();
        m1.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        m1.indices = vec![0, 1, 0];
        m1.uvs = vec![[0.0, 0.0], [1.0, 0.0]];
        m1.face_materials = vec![0];
        m1.face_tint_indices = vec![-1];

        let mut m2 = MeshData::new();
        // Co-located vertex at [1.0, 0.0, 0.0] along boundary!
        m2.positions = vec![[1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        m2.indices = vec![0, 1, 0];
        m2.uvs = vec![[0.0, 0.0], [1.0, 0.0]];
        m2.face_materials = vec![1];
        m2.face_tint_indices = vec![-1];

        let merged = MeshData::merge_welded_sections(&[&m1, &m2], 1e-4);
        // [1.0, 0.0, 0.0] should be welded into shared topology!
        assert_eq!(merged.vertex_count(), 3);
        assert_eq!(merged.positions[0], [0.0, 0.0, 0.0]);
        assert_eq!(merged.positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(merged.positions[2], [2.0, 0.0, 0.0]);
        // m2's index 0 should point to merged index 1
        assert_eq!(merged.indices, vec![0, 1, 0, 1, 2, 1]);
        assert_eq!(merged.face_materials, vec![0, 1]);
    }
}
