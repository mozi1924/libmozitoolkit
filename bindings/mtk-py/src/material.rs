use pyo3::prelude::*;
use pyo3::types::PyDict;

use mtk_material::{
    clean_icecube_name, clean_jmc2obj_name, decode_mineways_uv, is_mineways_atlas_name,
    lookup_swatch, remap_mesh_uvs_parallel, ImporterOrigin, MaterialResolver,
};

use crate::texture::PyBakedAtlas;

/// Python interface for Material Name Resolution and UV Remapping.
#[pyclass(name = "MaterialResolver")]
pub struct PyMaterialResolver;

#[pymethods]
impl PyMaterialResolver {
    /// Clean a raw jmc2obj material or texture name.
    #[staticmethod]
    pub fn clean_jmc2obj(name: &str) -> String {
        clean_jmc2obj_name(name)
    }

    /// Clean an Ice-Cube material or texture name.
    #[staticmethod]
    pub fn clean_icecube(name: &str) -> String {
        clean_icecube_name(name)
    }

    /// Check if a name belongs to Mineways terrain atlas.
    #[staticmethod]
    pub fn is_mineways_atlas(name: &str) -> bool {
        is_mineways_atlas_name(name)
    }

    /// Lookup Mineways swatch name by ID.
    #[staticmethod]
    pub fn lookup_mineways_swatch(swatch_id: usize) -> Option<(&'static str, &'static str)> {
        lookup_swatch(swatch_id)
    }

    /// Decode Mineways face UV to texture names and local [0, 1] UVs.
    #[staticmethod]
    pub fn decode_mineways_uv(
        u: f32,
        v: f32,
        width: u32,
        height: u32,
    ) -> (Option<String>, Option<String>, (f32, f32)) {
        let (pri, alt, local) = decode_mineways_uv(u, v, width, height);
        (
            pri.map(|s| s.to_string()),
            alt.map(|s| s.to_string()),
            (local[0], local[1]),
        )
    }

    /// Resolve a raw material name to its target sprite (resource_id, chunk_id, texture_id).
    #[staticmethod]
    pub fn resolve_material(
        name: &str,
        origin: &str,
        atlas: &PyBakedAtlas,
    ) -> Option<(String, u16, u32)> {
        let orig = ImporterOrigin::parse(origin);
        MaterialResolver::resolve(name, orig, &atlas.inner.address_map)
            .map(|(res, sp)| (res.as_string(), sp.chunk_id, sp.texture_id))
    }

    /// Batch parallel remapping of mesh UV loops and face material assignments.
    ///
    /// Args:
    ///     uvs: Flat list or array of floats [u0, v0, u1, v1, ...]
    ///     face_materials: List of material name strings per face
    ///     face_loop_ranges: List of (loop_start, loop_count) tuples per face
    ///     atlas: PyBakedAtlas reference
    ///     origin: Importer origin string ("auto", "jmc2obj", "mineways", "ice_cube", "generic")
    ///     mineways_size: Optional (width, height) for Mineways atlas decoding
    ///
    /// Returns:
    ///     dict containing:
    ///         "uvs": list of remapped float values
    ///         "face_chunk_ids": list of chunk ID integers per face
    ///         "face_texture_ids": list of texture ID integers per face
    ///         "unmapped_faces": count of unmapped faces
    #[staticmethod]
    #[pyo3(signature = (uvs, face_materials, face_loop_ranges, atlas, origin="auto", mineways_size=None))]
    pub fn remap_mesh_uvs(
        py: Python<'_>,
        uvs: Vec<f32>,
        face_materials: Vec<String>,
        face_loop_ranges: Vec<(u32, u32)>,
        atlas: &PyBakedAtlas,
        origin: &str,
        mineways_size: Option<(u32, u32)>,
    ) -> PyResult<PyObject> {
        let loop_count = uvs.len() / 2;
        let mut uv_pairs: Vec<[f32; 2]> = Vec::with_capacity(loop_count);
        for i in 0..loop_count {
            uv_pairs.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let orig = ImporterOrigin::parse(origin);
        let result = remap_mesh_uvs_parallel(
            &mut uv_pairs,
            &face_materials,
            &face_loop_ranges,
            &atlas.inner.address_map,
            orig,
            mineways_size,
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
}
