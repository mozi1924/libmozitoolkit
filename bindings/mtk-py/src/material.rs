use std::collections::HashMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use mtk_material::{
    clean_identifier, decode_grid_atlas_uv, remap_grid_atlas_uv_to_local,
    remap_mesh_multi_uvs_parallel, remap_mesh_uvs_parallel, GridAtlasSpec, MaterialResolver,
};

use crate::texture::PyBakedAtlas;

/// Python interface for Grid Atlas Specification.
#[pyclass(name = "GridAtlasSpec")]
#[derive(Clone)]
pub struct PyGridAtlasSpec {
    pub(crate) inner: GridAtlasSpec,
}

#[pymethods]
impl PyGridAtlasSpec {
    #[new]
    #[pyo3(signature = (swatch_size=18.0, tile_size=16.0, border=1.0, image_width=1024, image_height=1024, atlas_name_patterns=None, atlas_suffix_patterns=None, swatch_to_candidates=None))]
    pub fn new(
        swatch_size: f32,
        tile_size: f32,
        border: f32,
        image_width: u32,
        image_height: u32,
        atlas_name_patterns: Option<Vec<String>>,
        atlas_suffix_patterns: Option<Vec<String>>,
        swatch_to_candidates: Option<HashMap<usize, Vec<String>>>,
    ) -> Self {
        Self {
            inner: GridAtlasSpec {
                swatch_size,
                tile_size,
                border,
                image_width,
                image_height,
                atlas_name_patterns: atlas_name_patterns.unwrap_or_default(),
                atlas_suffix_patterns: atlas_suffix_patterns.unwrap_or_default(),
                swatch_to_candidates: swatch_to_candidates.unwrap_or_default(),
            },
        }
    }

    /// Set candidate texture names for a given swatch ID.
    pub fn set_swatch_candidates(&mut self, swatch_id: usize, candidates: Vec<String>) {
        self.inner.swatch_to_candidates.insert(swatch_id, candidates);
    }

    /// Check if a material name matches this grid atlas specification.
    pub fn matches_name(&self, name: &str) -> bool {
        self.inner.matches_atlas_name(name)
    }

    /// Decode UV on this grid atlas to candidate names and local UVs.
    pub fn decode_uv(&self, u: f32, v: f32) -> (Option<Vec<String>>, (f32, f32)) {
        let (cands, local) = decode_grid_atlas_uv(u, v, &self.inner);
        (cands.cloned(), (local[0], local[1]))
    }
}

/// Python interface for Material Name Resolution and UV Remapping.
#[pyclass(name = "MaterialResolver")]
pub struct PyMaterialResolver;

#[pymethods]
impl PyMaterialResolver {
    /// Clean a raw material or texture name into a normalized identifier stem.
    #[staticmethod]
    pub fn clean_name(name: &str) -> String {
        clean_identifier(name)
    }

    /// Convert a grid atlas UV coordinate (u, v) to its local [0, 1] swatch coordinate.
    #[staticmethod]
    pub fn remap_grid_atlas_uv_to_local(u: f32, v: f32, spec: &PyGridAtlasSpec) -> (f32, f32) {
        let uv = remap_grid_atlas_uv_to_local(u, v, &spec.inner);
        (uv[0], uv[1])
    }

    /// Resolve a raw material name to its target sprite (resource_id, chunk_id, texture_id) using optional external alias table.
    #[staticmethod]
    #[pyo3(signature = (name, atlas, aliases=None))]
    pub fn resolve_material(
        name: &str,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
    ) -> Option<(String, u16, u32)> {
        MaterialResolver::resolve(name, aliases.as_ref(), &atlas.inner.address_map)
            .map(|(res, sp)| (res.as_string(), sp.chunk_id, sp.texture_id))
    }

    /// Batch parallel remapping of mesh UV loops and face material assignments.
    ///
    /// Args:
    ///     uvs: Flat list or array of floats [u0, v0, u1, v1, ...]
    ///     face_materials: List of material name strings per face
    ///     face_loop_ranges: List of (loop_start, loop_count) tuples per face
    ///     atlas: PyBakedAtlas reference
    ///     aliases: Optional dictionary of material name -> candidate list
    ///     grid_atlas_spec: Optional PyGridAtlasSpec for grid atlas decoding
    ///
    /// Returns:
    ///     dict containing:
    ///         "uvs": list of remapped float values
    ///         "face_chunk_ids": list of chunk ID integers per face
    ///         "face_texture_ids": list of texture ID integers per face
    ///         "unmapped_faces": count of unmapped faces
    #[staticmethod]
    #[pyo3(signature = (uvs, face_materials, face_loop_ranges, atlas, aliases=None, grid_atlas_spec=None))]
    pub fn remap_mesh_uvs(
        py: Python<'_>,
        uvs: Vec<f32>,
        face_materials: Vec<String>,
        face_loop_ranges: Vec<(u32, u32)>,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
        grid_atlas_spec: Option<&PyGridAtlasSpec>,
    ) -> PyResult<PyObject> {
        let loop_count = uvs.len() / 2;
        let mut uv_pairs: Vec<[f32; 2]> = Vec::with_capacity(loop_count);
        for i in 0..loop_count {
            uv_pairs.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let result = remap_mesh_uvs_parallel(
            &mut uv_pairs,
            &face_materials,
            &face_loop_ranges,
            &atlas.inner.address_map,
            aliases.as_ref(),
            grid_atlas_spec.map(|s| &s.inner),
        );

        let mut flat_out_uvs = Vec::with_capacity(uv_pairs.len() * 2);
        for p in &uv_pairs {
            flat_out_uvs.push(p[0]);
            flat_out_uvs.push(p[1]);
        }

        let dict = PyDict::new(py);
        dict.set_item("uvs", flat_out_uvs)?;
        dict.set_item("face_chunk_ids", result.face_chunk_ids)?;
        dict.set_item("face_texture_ids", result.face_texture_ids)?;
        dict.set_item("unmapped_faces", result.unmapped_faces)?;
        dict.set_item("face_count", result.face_count)?;
        dict.set_item("loop_count", result.loop_count)?;

        Ok(dict.into())
    }

    /// Batch parallel multi-UV remapping of mesh loops (Atlas UV + Standalone/Local UV).
    ///
    /// Returns:
    ///     dict containing:
    ///         "atlas_uvs": list of remapped Atlas float values [u0, v0, u1, v1, ...]
    ///         "local_uvs": list of normalized local float values [u0, v0, u1, v1, ...]
    ///         "face_chunk_ids": list of chunk ID integers per face
    ///         "face_texture_ids": list of texture ID integers per face
    ///         "face_uv_modes": list of UV routing mode integers per face (0=Atlas, 1=Static, 2=Anim, 3=Overlay)
    ///         "face_is_overlay": list of boolean flags per face
    ///         "unmapped_faces": count of unmapped faces
    ///         "face_count": face count
    ///         "loop_count": loop count
    #[staticmethod]
    #[pyo3(signature = (uvs, face_materials, face_loop_ranges, atlas, aliases=None, grid_atlas_spec=None))]
    pub fn remap_mesh_multi_uvs(
        py: Python<'_>,
        uvs: Vec<f32>,
        face_materials: Vec<String>,
        face_loop_ranges: Vec<(u32, u32)>,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
        grid_atlas_spec: Option<&PyGridAtlasSpec>,
    ) -> PyResult<PyObject> {
        let loop_count = uvs.len() / 2;
        let mut uv_pairs: Vec<[f32; 2]> = Vec::with_capacity(loop_count);
        for i in 0..loop_count {
            uv_pairs.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let result = remap_mesh_multi_uvs_parallel(
            &uv_pairs,
            &face_materials,
            &face_loop_ranges,
            &atlas.inner.address_map,
            aliases.as_ref(),
            grid_atlas_spec.map(|s| &s.inner),
        );

        let mut flat_atlas_uvs = Vec::with_capacity(result.atlas_uvs.len() * 2);
        for p in &result.atlas_uvs {
            flat_atlas_uvs.push(p[0]);
            flat_atlas_uvs.push(p[1]);
        }

        let mut flat_local_uvs = Vec::with_capacity(result.local_uvs.len() * 2);
        for p in &result.local_uvs {
            flat_local_uvs.push(p[0]);
            flat_local_uvs.push(p[1]);
        }

        let dict = PyDict::new(py);
        dict.set_item("atlas_uvs", flat_atlas_uvs)?;
        dict.set_item("local_uvs", flat_local_uvs)?;
        dict.set_item("face_chunk_ids", result.face_chunk_ids)?;
        dict.set_item("face_texture_ids", result.face_texture_ids)?;
        dict.set_item("face_uv_modes", result.face_uv_modes)?;
        dict.set_item("face_is_overlay", result.face_is_overlay)?;
        dict.set_item("unmapped_faces", result.unmapped_faces)?;
        dict.set_item("face_count", result.face_count)?;
        dict.set_item("loop_count", result.loop_count)?;

        Ok(dict.into())
    }
}
