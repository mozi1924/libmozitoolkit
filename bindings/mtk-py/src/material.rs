use std::collections::HashMap;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use mtk_material::{
    clean_identifier, decode_grid_atlas_uv, remap_grid_atlas_uv_to_local,
    remap_mesh_multi_uvs_parallel, remap_mesh_uvs_parallel, GridAtlasSpec, MaterialResolver,
};

use crate::resource::PyResourcePackStack;
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

    #[getter]
    pub fn swatch_size(&self) -> f32 {
        self.inner.swatch_size
    }

    #[getter]
    pub fn tile_size(&self) -> f32 {
        self.inner.tile_size
    }

    #[getter]
    pub fn border(&self) -> f32 {
        self.inner.border
    }

    #[getter]
    pub fn image_width(&self) -> u32 {
        self.inner.image_width
    }

    #[getter]
    pub fn image_height(&self) -> u32 {
        self.inner.image_height
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
        dict.set_item("face_uv_transforms", result.face_uv_transforms)?;
        dict.set_item("face_uv_rotations", result.face_uv_rotations)?;
        dict.set_item("face_uv_modes", result.face_uv_modes)?;
        dict.set_item("face_is_overlay", result.face_is_overlay)?;
        dict.set_item("unmapped_faces", result.unmapped_faces)?;
        dict.set_item("face_count", result.face_count)?;
        dict.set_item("loop_count", result.loop_count)?;

        Ok(dict.into())
    }
}

/// Python wrapper for Rust BiomeResolver.
#[pyclass(name = "BiomeResolver")]
#[derive(Clone)]
pub struct PyBiomeResolver {
    pub(crate) inner: mtk_material::BiomeResolver,
}

#[pymethods]
impl PyBiomeResolver {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: mtk_material::BiomeResolver::new(),
        }
    }

    /// Load BiomeResolver from a JSON string.
    #[staticmethod]
    pub fn from_json(json_str: &str) -> PyResult<Self> {
        let inner = mtk_material::BiomeResolver::from_json(json_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Load BiomeResolver from a JSON file path.
    #[staticmethod]
    pub fn from_file(file_path: &str) -> PyResult<Self> {
        let inner = mtk_material::BiomeResolver::from_file(file_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Export mapping table to JSON string.
    pub fn to_json(&self) -> PyResult<String> {
        self.inner
            .to_json()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Load block models from directory.
    pub fn load_from_directory(&mut self, dir_path: &str) -> PyResult<()> {
        self.inner
            .load_from_directory(dir_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Load block models from .jar or .zip pack.
    pub fn load_from_zip(&mut self, zip_path: &str) -> PyResult<()> {
        self.inner
            .load_from_zip(zip_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Load block models from a ResourcePackStack.
    pub fn load_from_pack_stack(&mut self, stack: &PyResourcePackStack) -> PyResult<()> {
        self.inner.load_from_pack_stack(&stack.inner);
        Ok(())
    }

    /// Retrieve the paired overlay texture stem for a given base texture stem, if any.
    pub fn get_overlay_texture(&self, texture_stem: &str) -> Option<String> {
        self.inner.get_overlay_texture(texture_stem).map(|s| s.to_string())
    }

    /// Resolve tint metadata for a single texture.
    #[pyo3(signature = (texture_name, block_name=None, tint_index=None))]
    pub fn get_tint_info(
        &self,
        py: Python<'_>,
        texture_name: &str,
        block_name: Option<&str>,
        tint_index: Option<i32>,
    ) -> PyResult<PyObject> {
        let info = self.inner.get_tint_info(texture_name, block_name, tint_index);
        let dict = PyDict::new(py);
        dict.set_item("tint_type", info.tint_type)?;
        dict.set_item("tint_category", info.tint_category)?;
        dict.set_item("tint_weight", info.tint_weight)?;
        dict.set_item("base_tint_weight", info.base_tint_weight)?;
        dict.set_item("overlay_tint_weight", info.overlay_tint_weight)?;
        dict.set_item("has_overlay", info.has_overlay)?;
        dict.set_item("overlay_texture", info.overlay_texture)?;
        dict.set_item("is_hardcoded", info.is_hardcoded)?;
        dict.set_item("hardcoded_color", info.hardcoded_color)?;
        Ok(dict.into())
    }

    /// Compute batch mesh attributes in parallel across all faces.
    #[pyo3(signature = (face_texture_keys, biome_name="PLAINS", multi_biomes=None))]
    pub fn compute_biome_attributes(
        &self,
        py: Python<'_>,
        face_texture_keys: Vec<String>,
        biome_name: &str,
        multi_biomes: Option<Vec<(String, f32)>>,
    ) -> PyResult<PyObject> {
        let res = mtk_material::compute_mesh_biome_attributes(
            &face_texture_keys,
            biome_name,
            multi_biomes.as_deref(),
            &self.inner,
        );

        let dict = PyDict::new(py);
        dict.set_item("packed_tint_data", res.packed_tint_data)?;
        dict.set_item("tint_colors", res.tint_colors)?;
        dict.set_item("colormap_uvs", res.colormap_uvs)?;
        Ok(dict.into())
    }
}

/// Retrieve canonical metadata and colors for a single biome.
#[pyfunction]
#[pyo3(signature = (biome_name))]
pub fn get_biome_meta(py: Python<'_>, biome_name: &str) -> PyResult<PyObject> {
    let pal = mtk_material::get_biome_palette(biome_name);
    let dict = PyDict::new(py);
    dict.set_item("id", pal.id)?;
    dict.set_item("name", pal.name)?;
    dict.set_item("temperature", pal.temperature)?;
    dict.set_item("humidity", pal.humidity)?;
    dict.set_item("grass_hex", pal.grass_hex)?;
    dict.set_item("foliage_hex", pal.foliage_hex)?;
    dict.set_item("dry_foliage_hex", pal.dry_foliage_hex)?;
    dict.set_item("water_hex", pal.water_hex)?;
    dict.set_item("grass_linear", pal.grass_linear())?;
    dict.set_item("foliage_linear", pal.foliage_linear())?;
    dict.set_item("dry_foliage_linear", pal.dry_foliage_linear())?;
    dict.set_item("water_linear", pal.water_linear())?;
    dict.set_item("colormap_uv", pal.colormap_uv())?;
    dict.set_item("has_custom_grass", pal.has_custom_grass)?;
    dict.set_item("has_custom_foliage", pal.has_custom_foliage)?;
    dict.set_item("has_custom_dry_foliage", pal.has_custom_dry_foliage)?;
    Ok(dict.into())
}

/// Retrieve a list of all canonical vanilla biomes.
#[pyfunction]
pub fn get_all_biomes(py: Python<'_>) -> PyResult<PyObject> {
    let list = pyo3::types::PyList::empty(py);
    for pal in mtk_material::CANONICAL_BIOMES {
        let dict = PyDict::new(py);
        dict.set_item("id", pal.id)?;
        dict.set_item("name", pal.name)?;
        dict.set_item("temperature", pal.temperature)?;
        dict.set_item("humidity", pal.humidity)?;
        dict.set_item("grass_hex", pal.grass_hex)?;
        dict.set_item("foliage_hex", pal.foliage_hex)?;
        dict.set_item("dry_foliage_hex", pal.dry_foliage_hex)?;
        dict.set_item("water_hex", pal.water_hex)?;
        dict.set_item("grass_linear", pal.grass_linear())?;
        dict.set_item("foliage_linear", pal.foliage_linear())?;
        dict.set_item("dry_foliage_linear", pal.dry_foliage_linear())?;
        dict.set_item("water_linear", pal.water_linear())?;
        dict.set_item("colormap_uv", pal.colormap_uv())?;
        dict.set_item("has_custom_grass", pal.has_custom_grass)?;
        dict.set_item("has_custom_foliage", pal.has_custom_foliage)?;
        dict.set_item("has_custom_dry_foliage", pal.has_custom_dry_foliage)?;
        list.append(dict)?;
    }
    Ok(list.into())
}

/// Standalone batch function to compute mesh biome attributes in Rust parallel Rayon.
#[pyfunction]
#[pyo3(signature = (face_texture_keys, biome_name="PLAINS", multi_biomes=None, resolver=None))]
pub fn compute_biome_tint_attributes(
    py: Python<'_>,
    face_texture_keys: Vec<String>,
    biome_name: &str,
    multi_biomes: Option<Vec<(String, f32)>>,
    resolver: Option<&PyBiomeResolver>,
) -> PyResult<PyObject> {
    let default_resolver = mtk_material::BiomeResolver::new();
    let res_ref = resolver.map(|r| &r.inner).unwrap_or(&default_resolver);

    let res = mtk_material::compute_mesh_biome_attributes(
        &face_texture_keys,
        biome_name,
        multi_biomes.as_deref(),
        res_ref,
    );

    let dict = PyDict::new(py);
    dict.set_item("packed_tint_data", res.packed_tint_data)?;
    dict.set_item("tint_colors", res.tint_colors)?;
    dict.set_item("colormap_uvs", res.colormap_uvs)?;
    Ok(dict.into())
}
