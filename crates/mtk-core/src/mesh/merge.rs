//! # Mesh Merging Algorithms
//!
//! Provides batch merging of multiple `MeshData` buffers into a single unified mesh,
//! including zero-copy reference merging and streaming single-pass boundary welding.

use std::collections::HashMap;

use super::MeshData;
use crate::attributes::{AttributeData, AttributeDomain, MeshAttribute};

/// Pre-seeds the merged attribute map with empty buffers sized for the final face count.
///
/// Without this, each `extend_from_slice` during the per-section loop grows the
/// destination vectors incrementally (allocating and copying repeatedly).
fn seed_custom_attributes(
    target: &mut HashMap<String, MeshAttribute>,
    meshes: &[&MeshData],
    total_faces: usize,
) {
    for &m in meshes {
        for (name, attr) in &m.custom_attributes {
            if target.contains_key(name) {
                continue;
            }
            let capacity = if attr.domain == AttributeDomain::Face {
                total_faces
            } else {
                attr.len()
            };
            target.insert(
                name.clone(),
                MeshAttribute::new(
                    name.clone(),
                    attr.domain,
                    attr.data.empty_with_capacity(capacity),
                ),
            );
        }
    }
}

impl MeshData {
    /// Helper to merge custom attributes from `other` into `target`.
    pub(crate) fn extend_custom_attributes(
        target: &mut HashMap<String, MeshAttribute>,
        other: &HashMap<String, MeshAttribute>,
    ) {
        for (name, attr) in other {
            if let Some(existing) = target.get_mut(name) {
                if existing.domain == attr.domain {
                    match (&mut existing.data, &attr.data) {
                        (AttributeData::Float(a), AttributeData::Float(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::Float2(a), AttributeData::Float2(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::Float3(a), AttributeData::Float3(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::Float4(a), AttributeData::Float4(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::Int8(a), AttributeData::Int8(b)) => a.extend_from_slice(b),
                        (AttributeData::Int16(a), AttributeData::Int16(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::Int32(a), AttributeData::Int32(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::UInt8(a), AttributeData::UInt8(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::UInt16(a), AttributeData::UInt16(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::UInt32(a), AttributeData::UInt32(b)) => {
                            a.extend_from_slice(b)
                        }
                        (AttributeData::Bool(a), AttributeData::Bool(b)) => a.extend_from_slice(b),
                        (AttributeData::String(a), AttributeData::String(b)) => {
                            a.extend_from_slice(b)
                        }
                        _ => {}
                    }
                }
            } else {
                target.insert(name.clone(), attr.clone());
            }
        }
    }

    /// Efficiently merges multiple `MeshData` references into a single unified `MeshData`,
    /// pre-allocating exact capacities upfront to eliminate vector reallocations and cloning.
    pub fn merge_all_refs(meshes: &[&MeshData]) -> Self {
        if meshes.is_empty() {
            return Self::new();
        }
        if meshes.len() == 1 {
            return (*meshes[0]).clone();
        }

        let mut total_verts = 0;
        let mut total_normals = 0;
        let mut total_uvs = 0;
        let mut total_indices = 0;
        let mut total_quads = 0;
        let mut total_faces = 0;
        let mut has_sec_uvs = false;
        let mut has_colors = false;
        let mut has_custom_attrs = false;

        for &m in meshes {
            total_verts += m.positions.len();
            total_normals += m.normals.len();
            total_uvs += m.uvs.len();
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
            if !m.custom_attributes.is_empty() {
                has_custom_attrs = true;
            }
        }

        let uv_layout = meshes
            .iter()
            .map(|m| m.uv_layout)
            .find(|layout| *layout != super::UvLayout::None)
            .unwrap_or_default();
        let mut merged = Self {
            positions: Vec::with_capacity(total_verts),
            normals: Vec::with_capacity(total_normals),
            uvs: Vec::with_capacity(total_uvs),
            uv_layout,
            secondary_uvs: if has_sec_uvs {
                Some(Vec::with_capacity(total_verts))
            } else {
                None
            },
            colors: if has_colors {
                Some(Vec::with_capacity(total_verts))
            } else {
                None
            },
            indices: Vec::with_capacity(total_indices),
            quad_indices: if total_quads > 0 {
                Some(Vec::with_capacity(total_quads))
            } else {
                None
            },
            face_materials: Vec::with_capacity(total_faces),
            face_tint_indices: Vec::with_capacity(total_faces),
            custom_attributes: HashMap::new(),
        };

        if has_custom_attrs {
            seed_custom_attributes(&mut merged.custom_attributes, meshes, total_faces);
        }

        for &m in meshes {
            let base_idx = merged.positions.len() as u32;
            merged.positions.extend_from_slice(&m.positions);
            merged.normals.extend_from_slice(&m.normals);
            merged.uvs.extend_from_slice(&m.uvs);

            let old_idx_len = merged.indices.len();
            merged.indices.extend_from_slice(&m.indices);
            for idx in &mut merged.indices[old_idx_len..] {
                *idx += base_idx;
            }

            if let Some(ref o_quads) = m.quad_indices {
                let quads = merged.quad_indices.get_or_insert_with(Vec::new);
                let old_quad_len = quads.len();
                quads.extend_from_slice(o_quads);
                for idx in &mut quads[old_quad_len..] {
                    *idx += base_idx;
                }
            }

            merged.face_materials.extend_from_slice(&m.face_materials);
            merged
                .face_tint_indices
                .extend_from_slice(&m.face_tint_indices);

            if let Some(ref other_sec) = m.secondary_uvs {
                let sec = merged.secondary_uvs.get_or_insert_with(Vec::new);
                sec.extend_from_slice(other_sec);
            }

            if has_colors {
                let col = merged.colors.get_or_insert_with(Vec::new);
                let expected_count = if let Some(ref q) = m.quad_indices {
                    q.len()
                } else if !m.uvs.is_empty() {
                    m.uvs.len()
                } else {
                    m.positions.len()
                };
                if let Some(ref other_col) = m.colors {
                    col.extend_from_slice(other_col);
                    while col.len() < merged.uvs.len().max(merged.positions.len()) {
                        col.push([1.0, 1.0, 1.0, 1.0]);
                    }
                } else {
                    col.resize(col.len() + expected_count, [1.0, 1.0, 1.0, 1.0]);
                }
            }

            if has_custom_attrs {
                Self::extend_custom_attributes(&mut merged.custom_attributes, &m.custom_attributes);
            }
        }

        merged
    }

    /// Merges multiple internally-welded Section meshes into a single seamless unified mesh,
    /// deduplicating only co-located boundary vertices along chunk interfaces in a single streaming pass.
    ///
    /// Eliminates intermediate buffer allocations and duplicate full-mesh spatial hash iterations.
    pub fn merge_welded_sections(meshes: &[&MeshData], tolerance: f32) -> Self {
        Self::merge_welded_sections_with(meshes, tolerance, |_| true)
    }

    /// Merges multiple internally-welded Section meshes, welding only the vertices for which
    /// `is_weld_candidate` returns `true`.
    ///
    /// The closure receives the quantized spatial cell `[round(x / tolerance), ..]` used as the
    /// deduplication key, so callers can restrict welding to positions shared between Sections
    /// (16-block boundary planes) instead of hashing every interior vertex. Vertices rejected by
    /// the predicate stay unique, which is safe when each Section mesh is already internally
    /// welded: only positions on a Section boundary can be duplicated by a neighbouring Section.
    pub fn merge_welded_sections_filtered<F>(
        meshes: &[&MeshData],
        tolerance: f32,
        is_weld_candidate: F,
    ) -> Self
    where
        F: Fn([i32; 3]) -> bool,
    {
        Self::merge_welded_sections_with(meshes, tolerance, is_weld_candidate)
    }

    fn merge_welded_sections_with<F>(
        meshes: &[&MeshData],
        tolerance: f32,
        is_weld_candidate: F,
    ) -> Self
    where
        F: Fn([i32; 3]) -> bool,
    {
        if meshes.is_empty() {
            return Self::new();
        }
        if meshes.len() == 1 {
            return (*meshes[0]).clone();
        }
        if tolerance <= 0.0 {
            return Self::merge_all_refs(meshes);
        }

        let mut total_verts = 0;
        let mut total_normals = 0;
        let mut total_uvs = 0;
        let mut total_indices = 0;
        let mut total_quads = 0;
        let mut total_faces = 0;
        let mut has_sec_uvs = false;
        let mut has_colors = false;
        let mut has_custom_attrs = false;

        for &m in meshes {
            total_verts += m.positions.len();
            total_normals += m.normals.len();
            total_uvs += m.uvs.len();
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
            if !m.custom_attributes.is_empty() {
                has_custom_attrs = true;
            }
        }

        let uv_layout = meshes
            .iter()
            .map(|m| m.uv_layout)
            .find(|layout| *layout != super::UvLayout::None)
            .unwrap_or_default();
        let mut merged = Self {
            positions: Vec::with_capacity(total_verts),
            normals: Vec::with_capacity(total_normals),
            uvs: Vec::with_capacity(total_uvs),
            uv_layout,
            secondary_uvs: if has_sec_uvs {
                Some(Vec::with_capacity(total_verts))
            } else {
                None
            },
            colors: if has_colors {
                Some(Vec::with_capacity(total_verts))
            } else {
                None
            },
            indices: Vec::with_capacity(total_indices),
            quad_indices: if total_quads > 0 {
                Some(Vec::with_capacity(total_quads))
            } else {
                None
            },
            face_materials: Vec::with_capacity(total_faces),
            face_tint_indices: Vec::with_capacity(total_faces),
            custom_attributes: HashMap::new(),
        };

        if has_custom_attrs {
            seed_custom_attributes(&mut merged.custom_attributes, meshes, total_faces);
        }

        let inv_dist = 1.0 / tolerance;
        // Spatial hash map mapping discrete grid position [i32; 3] to global vertex index in merged.positions.
        let mut coord_map = rustc_hash::FxHashMap::<[i32; 3], u32>::with_capacity_and_hasher(
            (total_verts / 4).max(64),
            Default::default(),
        );

        let mut local_remap = Vec::with_capacity(2048);

        for &m in meshes {
            if m.positions.is_empty() {
                continue;
            }

            local_remap.clear();
            local_remap.reserve(m.positions.len());

            for (i, &p) in m.positions.iter().enumerate() {
                let key = [
                    (p[0] * inv_dist).round() as i32,
                    (p[1] * inv_dist).round() as i32,
                    (p[2] * inv_dist).round() as i32,
                ];

                if is_weld_candidate(key) {
                    if let Some(&existing_idx) = coord_map.get(&key) {
                        local_remap.push(existing_idx);
                        continue;
                    }
                    let new_idx = merged.positions.len() as u32;
                    coord_map.insert(key, new_idx);
                    local_remap.push(new_idx);
                } else {
                    local_remap.push(merged.positions.len() as u32);
                }

                merged.positions.push(p);
                if let Some(&norm) = m.normals.get(i) {
                    merged.normals.push(norm);
                }
            }

            // Remap and append triangle indices directly in a single streaming pass
            merged.indices.reserve(m.indices.len());
            for &idx in &m.indices {
                if let Some(&new_i) = local_remap.get(idx as usize) {
                    merged.indices.push(new_i);
                } else {
                    merged.indices.push(idx);
                }
            }

            // Remap and append quad indices directly in a single pass
            if let Some(ref o_quads) = m.quad_indices {
                let quads = merged.quad_indices.get_or_insert_with(Vec::new);
                quads.reserve(o_quads.len());
                for &idx in o_quads {
                    if let Some(&new_i) = local_remap.get(idx as usize) {
                        quads.push(new_i);
                    } else {
                        quads.push(idx);
                    }
                }
            }

            // Direct contiguous memory copies for per-loop and per-face attributes
            merged.uvs.extend_from_slice(&m.uvs);
            merged.face_materials.extend_from_slice(&m.face_materials);
            merged
                .face_tint_indices
                .extend_from_slice(&m.face_tint_indices);

            if let Some(ref other_sec) = m.secondary_uvs {
                let sec = merged.secondary_uvs.get_or_insert_with(Vec::new);
                sec.extend_from_slice(other_sec);
            }

            if has_colors {
                let col = merged.colors.get_or_insert_with(Vec::new);
                let expected_count = if let Some(ref q) = m.quad_indices {
                    q.len()
                } else if !m.uvs.is_empty() {
                    m.uvs.len()
                } else {
                    m.positions.len()
                };
                if let Some(ref other_col) = m.colors {
                    col.extend_from_slice(other_col);
                    while col.len() < merged.uvs.len().max(merged.positions.len()) {
                        col.push([1.0, 1.0, 1.0, 1.0]);
                    }
                } else {
                    col.resize(col.len() + expected_count, [1.0, 1.0, 1.0, 1.0]);
                }
            }

            if has_custom_attrs {
                Self::extend_custom_attributes(&mut merged.custom_attributes, &m.custom_attributes);
            }
        }

        merged
    }
}
