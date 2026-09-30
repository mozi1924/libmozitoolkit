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

        self.indices.reserve(other.indices.len());
        for &idx in &other.indices {
            self.indices.push(base_idx + idx);
        }

        if let Some(ref o_quads) = other.quad_indices {
            let quads = self.quad_indices.get_or_insert_with(Vec::new);
            quads.reserve(o_quads.len());
            for &idx in o_quads {
                quads.push(base_idx + idx);
            }
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

    /// Efficiently merges multiple `MeshData` buffers into a single unified `MeshData`,
    /// pre-allocating exact capacities upfront to eliminate vector reallocations.
    pub fn merge_all(meshes: &[MeshData]) -> Self {
        if meshes.is_empty() {
            return Self::new();
        }
        if meshes.len() == 1 {
            return meshes[0].clone();
        }

        let mut total_verts = 0;
        let mut total_indices = 0;
        let mut total_quads = 0;
        let mut total_faces = 0;
        let mut has_sec_uvs = false;
        let mut has_colors = false;

        for m in meshes {
            total_verts += m.positions.len();
            total_indices += m.indices.len();
            if let Some(ref q) = m.quad_indices {
                total_quads += q.len();
            }
            total_faces += m.face_materials.len();
            if m.secondary_uvs.is_some() {
                has_sec_uvs = true;
            }
            if m.colors.is_some() {
                has_colors = true;
            }
        }

        let mut merged = Self {
            positions: Vec::with_capacity(total_verts),
            normals: Vec::with_capacity(total_verts),
            uvs: Vec::with_capacity(total_verts),
            secondary_uvs: if has_sec_uvs { Some(Vec::with_capacity(total_verts)) } else { None },
            colors: if has_colors { Some(Vec::with_capacity(total_verts)) } else { None },
            indices: Vec::with_capacity(total_indices),
            quad_indices: if total_quads > 0 { Some(Vec::with_capacity(total_quads)) } else { None },
            face_materials: Vec::with_capacity(total_faces),
            face_tint_indices: Vec::with_capacity(total_faces),
            custom_attributes: HashMap::new(),
        };

        for m in meshes {
            merged.append_mesh(m);
        }

        merged
    }

    /// Welds co-located vertices within `tolerance` distance into shared topology,
    /// remapping indices while preserving per-corner loop UVs and colors.
    pub fn weld_spatial_vertices(&mut self, tolerance: f32) {
        if self.positions.is_empty() || tolerance <= 0.0 {
            return;
        }

        let inv_dist = 1.0 / tolerance;
        let mut coord_map: HashMap<[i32; 3], u32> = HashMap::with_capacity(self.positions.len());
        let mut remap: Vec<u32> = Vec::with_capacity(self.positions.len());
        let mut new_positions: Vec<[f32; 3]> = Vec::with_capacity(self.positions.len());
        let mut new_normals: Vec<[f32; 3]> = Vec::with_capacity(self.normals.len());

        for (i, &p) in self.positions.iter().enumerate() {
            let key = [
                (p[0] * inv_dist).round() as i32,
                (p[1] * inv_dist).round() as i32,
                (p[2] * inv_dist).round() as i32,
            ];

            if let Some(&existing_idx) = coord_map.get(&key) {
                remap.push(existing_idx);
            } else {
                let new_idx = new_positions.len() as u32;
                coord_map.insert(key, new_idx);
                remap.push(new_idx);
                new_positions.push(p);
                if let Some(&norm) = self.normals.get(i) {
                    new_normals.push(norm);
                }
            }
        }

        // Remap triangle/polygon indices
        #[cfg(feature = "parallel")]
        {
            use rayon::prelude::*;
            self.indices.par_iter_mut().for_each(|idx| {
                if let Some(&new_i) = remap.get(*idx as usize) {
                    *idx = new_i;
                }
            });

            if let Some(ref mut quads) = self.quad_indices {
                quads.par_iter_mut().for_each(|idx| {
                    if let Some(&new_i) = remap.get(*idx as usize) {
                        *idx = new_i;
                    }
                });
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            for idx in &mut self.indices {
                if let Some(&new_i) = remap.get(*idx as usize) {
                    *idx = new_i;
                }
            }

            if let Some(ref mut quads) = self.quad_indices {
                for idx in quads {
                    if let Some(&new_i) = remap.get(*idx as usize) {
                        *idx = new_i;
                    }
                }
            }
        }

        self.positions = new_positions;
        if !self.normals.is_empty() {
            self.normals = new_normals;
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

    /// Populates all standard Face attributes from an ordered slice of `FaceAttributes`.
    pub fn populate_standard_face_attributes(&mut self, face_attrs: &[FaceAttributes]) {
        use crate::attributes::constants::*;
        use crate::attributes::{AttributeData, AttributeDomain, MeshAttribute};

        let face_count = face_attrs.len();
        if face_count == 0 {
            return;
        }

        let mut textures = Vec::with_capacity(face_count);
        let mut face_dirs = Vec::with_capacity(face_count);
        let mut emissions = Vec::with_capacity(face_count);
        let mut mat_props = Vec::with_capacity(face_count);
        let mut uv_transforms = Vec::with_capacity(face_count);
        let mut uv_rotations = Vec::with_capacity(face_count);
        let mut uv_modes = Vec::with_capacity(face_count);
        let mut anim_timings = Vec::with_capacity(face_count);
        let mut anim_sizes = Vec::with_capacity(face_count);
        let mut tint_colors = Vec::with_capacity(face_count);
        let mut tint_datas = Vec::with_capacity(face_count);
        let mut colormap_uvs = Vec::with_capacity(face_count);
        let mut block_x = Vec::with_capacity(face_count);
        let mut block_y = Vec::with_capacity(face_count);
        let mut block_z = Vec::with_capacity(face_count);
        let mut chunk_ids = Vec::with_capacity(face_count);
        let mut has_chunk_id = false;
        let mut texture_ids = Vec::with_capacity(face_count);
        let mut has_texture_id = false;

        for attr in face_attrs {
            textures.push(attr.texture_key.clone());
            face_dirs.push(attr.face_dir as i32);
            emissions.push(attr.emission);
            mat_props.push(attr.material_props);
            uv_transforms.push(attr.uv_transform);
            uv_rotations.push(attr.uv_rotation);
            uv_modes.push(attr.uv_mode);
            anim_timings.push(attr.anim_timing);
            anim_sizes.push(attr.anim_frame_size);
            tint_colors.push(attr.biome_tint_color);
            tint_datas.push(attr.biome_tint_data);
            colormap_uvs.push(attr.colormap_uv);
            block_x.push(attr.block_pos[0]);
            block_y.push(attr.block_pos[1]);
            block_z.push(attr.block_pos[2]);

            if let Some(cid) = attr.atlas_chunk_id {
                has_chunk_id = true;
                chunk_ids.push(cid as i32);
            } else {
                chunk_ids.push(0);
            }

            if let Some(tid) = attr.atlas_texture_id {
                has_texture_id = true;
                texture_ids.push(tid);
            } else {
                texture_ids.push(0);
            }
        }

        // Modern canonical attributes
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_SOURCE_TEXTURE,
            AttributeDomain::Face,
            AttributeData::String(textures.clone()),
        ));
        // Backwards-compatible alias for existing shaders/scripts
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_SOURCE_TEXTURE_KEY,
            AttributeDomain::Face,
            AttributeData::String(textures),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_FACE_DIR,
            AttributeDomain::Face,
            AttributeData::Int32(face_dirs),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_EMISSION,
            AttributeDomain::Face,
            AttributeData::Float(emissions),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_MATERIAL_PROPS,
            AttributeDomain::Face,
            AttributeData::Float4(mat_props),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_TRANSFORM,
            AttributeDomain::Face,
            AttributeData::Float4(uv_transforms.clone()),
        ));
        // Legacy alias mtk_uv_tiling_transform
        self.add_custom_attribute(MeshAttribute::new(
            "mtk_uv_tiling_transform",
            AttributeDomain::Face,
            AttributeData::Float4(uv_transforms),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_ROTATION,
            AttributeDomain::Face,
            AttributeData::Float(uv_rotations),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_UV_MODE,
            AttributeDomain::Face,
            AttributeData::UInt8(uv_modes),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_ANIM_TIMING,
            AttributeDomain::Face,
            AttributeData::Float3(anim_timings),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_ANIM_FRAME_SIZE,
            AttributeDomain::Face,
            AttributeData::Float3(anim_sizes),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BIOME_TINT_COLOR,
            AttributeDomain::Face,
            AttributeData::Float4(tint_colors),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BIOME_TINT_DATA,
            AttributeDomain::Face,
            AttributeData::Float4(tint_datas),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_COLORMAP_UV,
            AttributeDomain::Face,
            AttributeData::Float3(colormap_uvs),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_X,
            AttributeDomain::Face,
            AttributeData::Int32(block_x),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_Y,
            AttributeDomain::Face,
            AttributeData::Int32(block_y),
        ));
        self.add_custom_attribute(MeshAttribute::new(
            ATTR_BLOCK_Z,
            AttributeDomain::Face,
            AttributeData::Int32(block_z),
        ));

        if has_chunk_id {
            self.add_custom_attribute(MeshAttribute::new(
                ATTR_ATLAS_CHUNK_ID,
                AttributeDomain::Face,
                AttributeData::Int32(chunk_ids),
            ));
        }

        if has_texture_id {
            self.add_custom_attribute(MeshAttribute::new(
                ATTR_ATLAS_TEXTURE_ID,
                AttributeDomain::Face,
                AttributeData::UInt32(texture_ids),
            ));
        }
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
