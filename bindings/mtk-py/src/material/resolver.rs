//! MaterialResolver and BiomeResolver Python bindings.

use std::collections::HashMap;

use crate::buffer::{
    create_owned_f32_memoryview, create_owned_i32_memoryview, create_owned_u32_memoryview,
};
use mtk_material::{
    clean_identifier, remap_grid_atlas_uv_to_local, remap_mesh_multi_uvs_paletted_with_options,
    remap_mesh_multi_uvs_parallel_with_options, remap_mesh_uvs_parallel, reroute_mesh_animation,
    resolve_mesh_material_routes, BiomeResolver, MaterialResolver,
};
use pyo3::buffer::PyBuffer;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::resource::PyResourcePackStack;
use crate::texture::PyBakedAtlas;

use super::atlas::PyGridAtlasSpec;

enum BorrowedU32Buffer {
    Py(PyBuffer<u32>),
    Owned(Vec<u32>),
}

impl BorrowedU32Buffer {
    fn extract(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(buf) = PyBuffer::<u32>::get(obj) {
            if buf.is_c_contiguous() {
                return Ok(Self::Py(buf));
            }
        }
        Ok(Self::Owned(obj.extract::<Vec<u32>>()?))
    }

    fn as_slice(&self) -> &[u32] {
        match self {
            Self::Py(buf) => unsafe {
                std::slice::from_raw_parts(buf.buf_ptr() as *const u32, buf.item_count())
            },
            Self::Owned(vec) => vec.as_slice(),
        }
    }
}

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

        let result = py.allow_threads(|| {
            remap_mesh_uvs_parallel(
                &mut uv_pairs,
                &face_materials,
                &face_loop_ranges,
                &atlas.inner.address_map,
                aliases.as_ref(),
                grid_atlas_spec.map(|s| &s.inner),
            )
        });

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
    ///         "face_is_animated": list of boolean flags per face
    ///         "face_anim_timing": flat list of [total_frames, frametime, interpolate] per face
    ///         "face_anim_frame_size": flat list of [frame_width, frame_height, 0] per face
    ///         "unmapped_faces": count of unmapped faces
    ///         "face_count": face count
    ///         "loop_count": loop count
    ///     enable_animated_textures: Route multi-frame sprites to dedicated animated atlas chunks.
    #[staticmethod]
    #[pyo3(signature = (uvs, face_materials, face_loop_ranges, atlas, aliases=None, grid_atlas_spec=None, enable_animated_textures=false))]
    pub fn remap_mesh_multi_uvs(
        py: Python<'_>,
        uvs: Vec<f32>,
        face_materials: Vec<String>,
        face_loop_ranges: Vec<(u32, u32)>,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
        grid_atlas_spec: Option<&PyGridAtlasSpec>,
        enable_animated_textures: bool,
    ) -> PyResult<PyObject> {
        let loop_count = uvs.len() / 2;
        let mut uv_pairs: Vec<[f32; 2]> = Vec::with_capacity(loop_count);
        for i in 0..loop_count {
            uv_pairs.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let result = py.allow_threads(|| {
            remap_mesh_multi_uvs_parallel_with_options(
                &uv_pairs,
                &face_materials,
                &face_loop_ranges,
                &atlas.inner.address_map,
                aliases.as_ref(),
                grid_atlas_spec.map(|s| &s.inner),
                enable_animated_textures,
            )
        });

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
        dict.set_item("face_is_animated", result.face_is_animated)?;
        dict.set_item("face_anim_timing", result.face_anim_timing)?;
        dict.set_item("face_anim_frame_size", result.face_anim_frame_size)?;
        dict.set_item("unmapped_faces", result.unmapped_faces)?;
        dict.set_item("face_count", result.face_count)?;
        dict.set_item("loop_count", result.loop_count)?;

        Ok(dict.into())
    }

    /// Resolve atlas routing (chunk/texture/animation metadata) for face texture keys.
    #[staticmethod]
    #[pyo3(signature = (face_texture_keys, atlas, aliases=None, enable_animated_textures=false))]
    pub fn resolve_mesh_routes(
        py: Python<'_>,
        face_texture_keys: Vec<String>,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
        enable_animated_textures: bool,
    ) -> PyResult<PyObject> {
        let routes = py.allow_threads(|| {
            resolve_mesh_material_routes(
                &face_texture_keys,
                aliases.as_ref(),
                &atlas.inner.address_map,
                enable_animated_textures,
            )
        });

        let mut chunk_ids = Vec::with_capacity(routes.len());
        let mut texture_ids = Vec::with_capacity(routes.len());
        let mut is_animated = Vec::with_capacity(routes.len());
        let mut anim_timing = Vec::with_capacity(routes.len() * 3);
        let mut anim_frame_size = Vec::with_capacity(routes.len() * 3);
        let mut mapped = 0usize;
        for route in &routes {
            chunk_ids.push(route.chunk_id);
            texture_ids.push(route.texture_id);
            is_animated.push(route.is_animated);
            anim_timing.extend_from_slice(&route.anim_timing);
            anim_frame_size.extend_from_slice(&route.anim_frame_size);
            if route.mapped {
                mapped += 1;
            }
        }

        let dict = PyDict::new(py);
        dict.set_item("chunk_ids", chunk_ids)?;
        dict.set_item("texture_ids", texture_ids)?;
        dict.set_item("is_animated", is_animated)?;
        dict.set_item("anim_timing", anim_timing)?;
        dict.set_item("anim_frame_size", anim_frame_size)?;
        dict.set_item("mapped_faces", mapped)?;
        dict.set_item("unmapped_faces", routes.len().saturating_sub(mapped))?;
        Ok(dict.into())
    }

    /// Re-project existing atlas UVs from the previous chunk routing to the current animation policy.
    #[staticmethod]
    #[pyo3(signature = (uvs, face_loop_ranges, face_texture_keys, face_old_chunk_ids, atlas, aliases=None, enable_animated_textures=false))]
    pub fn reroute_mesh_animation(
        py: Python<'_>,
        uvs: Vec<f32>,
        face_loop_ranges: Vec<(u32, u32)>,
        face_texture_keys: Vec<String>,
        face_old_chunk_ids: Vec<u16>,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
        enable_animated_textures: bool,
    ) -> PyResult<PyObject> {
        let loop_count = uvs.len() / 2;
        let mut uv_pairs: Vec<[f32; 2]> = Vec::with_capacity(loop_count);
        for i in 0..loop_count {
            uv_pairs.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let result = py.allow_threads(|| {
            reroute_mesh_animation(
                &uv_pairs,
                &face_loop_ranges,
                &face_texture_keys,
                &face_old_chunk_ids,
                &atlas.inner.address_map,
                aliases.as_ref(),
                enable_animated_textures,
            )
        });

        let mut flat_atlas_uvs = Vec::with_capacity(result.atlas_uvs.len() * 2);
        for p in &result.atlas_uvs {
            flat_atlas_uvs.push(p[0]);
            flat_atlas_uvs.push(p[1]);
        }

        let dict = PyDict::new(py);
        dict.set_item("atlas_uvs", flat_atlas_uvs)?;
        dict.set_item("face_chunk_ids", result.face_chunk_ids)?;
        dict.set_item("face_texture_ids", result.face_texture_ids)?;
        dict.set_item("face_is_animated", result.face_is_animated)?;
        dict.set_item("face_anim_timing", result.face_anim_timing)?;
        dict.set_item("face_anim_frame_size", result.face_anim_frame_size)?;
        dict.set_item("unmapped_faces", result.unmapped_faces)?;
        dict.set_item("uv_unmapped_faces", result.uv_unmapped_faces)?;
        Ok(dict.into())
    }

    /// Palette-native multi-UV remap. Returns zero-copy buffers for per-face ids and metrics.
    #[staticmethod]
    #[pyo3(signature = (
        uvs,
        face_material_palette,
        face_material_indices,
        face_loop_ranges,
        atlas,
        aliases=None,
        grid_atlas_spec=None,
        existing_key_palette=None,
        existing_key_indices=None,
        enable_animated_textures=false
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn remap_mesh_multi_uvs_paletted(
        py: Python<'_>,
        uvs: Vec<f32>,
        face_material_palette: Vec<String>,
        face_material_indices: &Bound<'_, PyAny>,
        face_loop_ranges: Vec<(u32, u32)>,
        atlas: &PyBakedAtlas,
        aliases: Option<HashMap<String, Vec<String>>>,
        grid_atlas_spec: Option<&PyGridAtlasSpec>,
        existing_key_palette: Option<Vec<String>>,
        existing_key_indices: Option<&Bound<'_, PyAny>>,
        enable_animated_textures: bool,
    ) -> PyResult<PyObject> {
        let loop_count = uvs.len() / 2;
        let mut uv_pairs: Vec<[f32; 2]> = Vec::with_capacity(loop_count);
        for i in 0..loop_count {
            uv_pairs.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let material_indices = BorrowedU32Buffer::extract(face_material_indices)?;
        let existing_indices = existing_key_indices
            .map(BorrowedU32Buffer::extract)
            .transpose()?;
        let result = remap_mesh_multi_uvs_paletted_with_options(
            &uv_pairs,
            &face_material_palette,
            material_indices.as_slice(),
            &face_loop_ranges,
            &atlas.inner.address_map,
            aliases.as_ref(),
            grid_atlas_spec.map(|s| &s.inner),
            existing_key_palette.as_deref(),
            existing_indices.as_ref().map(|b| b.as_slice()),
            enable_animated_textures,
        );

        let mesh = result.mesh;
        let mut flat_atlas_uvs = Vec::with_capacity(mesh.atlas_uvs.len() * 2);
        for p in &mesh.atlas_uvs {
            flat_atlas_uvs.push(p[0]);
            flat_atlas_uvs.push(p[1]);
        }
        let mut flat_local_uvs = Vec::with_capacity(mesh.local_uvs.len() * 2);
        for p in &mesh.local_uvs {
            flat_local_uvs.push(p[0]);
            flat_local_uvs.push(p[1]);
        }
        let mut flat_transforms = Vec::with_capacity(mesh.face_uv_transforms.len() * 4);
        for t in &mesh.face_uv_transforms {
            flat_transforms.extend_from_slice(t);
        }
        let mut flat_anim_timing = Vec::with_capacity(mesh.face_anim_timing.len() * 3);
        for t in &mesh.face_anim_timing {
            flat_anim_timing.extend_from_slice(t);
        }
        let mut flat_anim_size = Vec::with_capacity(mesh.face_anim_frame_size.len() * 3);
        for s in &mesh.face_anim_frame_size {
            flat_anim_size.extend_from_slice(s);
        }

        let dict = PyDict::new(py);
        dict.set_item(
            "atlas_uvs",
            create_owned_f32_memoryview(py, flat_atlas_uvs)?,
        )?;
        dict.set_item(
            "local_uvs",
            create_owned_f32_memoryview(py, flat_local_uvs)?,
        )?;
        dict.set_item(
            "face_chunk_ids",
            create_owned_i32_memoryview(
                py,
                mesh.face_chunk_ids.iter().map(|v| *v as i32).collect(),
            )?,
        )?;
        dict.set_item(
            "face_texture_ids",
            create_owned_i32_memoryview(
                py,
                mesh.face_texture_ids.iter().map(|v| *v as i32).collect(),
            )?,
        )?;
        dict.set_item(
            "face_uv_transforms",
            create_owned_f32_memoryview(py, flat_transforms)?,
        )?;
        dict.set_item(
            "face_uv_rotations",
            create_owned_f32_memoryview(py, mesh.face_uv_rotations.clone())?,
        )?;
        dict.set_item("face_uv_modes", mesh.face_uv_modes)?;
        dict.set_item("face_is_overlay", mesh.face_is_overlay)?;
        dict.set_item("face_is_animated", mesh.face_is_animated)?;
        dict.set_item(
            "face_anim_timing",
            create_owned_f32_memoryview(py, flat_anim_timing)?,
        )?;
        dict.set_item(
            "face_anim_frame_size",
            create_owned_f32_memoryview(py, flat_anim_size)?,
        )?;
        dict.set_item("source_key_palette", result.source_key_palette)?;
        dict.set_item(
            "face_source_key_indices",
            create_owned_u32_memoryview(py, result.face_source_key_indices.clone())?,
        )?;
        dict.set_item("unmapped_faces", mesh.unmapped_faces)?;
        dict.set_item("face_count", mesh.face_count)?;
        dict.set_item("loop_count", mesh.loop_count)?;
        Ok(dict.into())
    }
}

/// Python wrapper for Rust BiomeResolver.
#[pyclass(name = "BiomeResolver")]
#[derive(Clone)]
pub struct PyBiomeResolver {
    pub(crate) inner: BiomeResolver,
}

fn py_dict_to_json_hashmap(
    py: Python<'_>,
    dict: &Bound<'_, pyo3::types::PyDict>,
) -> PyResult<HashMap<String, serde_json::Value>> {
    let json_module = py.import("json")?;
    let json_str: String = json_module.call_method1("dumps", (dict,))?.extract()?;
    let map: HashMap<String, serde_json::Value> = serde_json::from_str(&json_str)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    Ok(map)
}

#[pymethods]
impl PyBiomeResolver {
    #[new]
    #[pyo3(signature = (models=None))]
    pub fn new(py: Python<'_>, models: Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<Self> {
        let mut inner = mtk_material::BiomeResolver::new();
        if let Some(m) = models {
            let map = py_dict_to_json_hashmap(py, &m)?;
            inner.set_models(map);
        }
        Ok(Self { inner })
    }

    /// Set loaded block models dictionary.
    pub fn set_models(
        &mut self,
        py: Python<'_>,
        models: Bound<'_, pyo3::types::PyDict>,
    ) -> PyResult<()> {
        let map = py_dict_to_json_hashmap(py, &models)?;
        self.inner.set_models(map);
        Ok(())
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
        self.inner
            .get_overlay_texture(texture_stem)
            .map(|s| s.to_string())
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
        let info = self
            .inner
            .get_tint_info(texture_name, block_name, tint_index);
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
        dict.set_item("hardcoded_hex", info.hardcoded_hex)?;
        dict.set_item("default_tint_weight", info.tint_weight)?;
        dict.set_item("default_base_tint_weight", info.base_tint_weight)?;
        dict.set_item("default_overlay_tint_weight", info.overlay_tint_weight)?;
        Ok(dict.into())
    }

    /// Compute batch mesh attributes in parallel across all faces.
    #[pyo3(signature = (
        face_texture_keys,
        biome_name="PLAINS",
        multi_biomes=None,
        custom_temp=None,
        custom_humidity=None,
        custom_grass=None,
        custom_foliage=None,
        custom_dry_foliage=None,
        custom_water=None,
        has_custom_grass=false,
        has_custom_foliage=false,
        has_custom_dry_foliage=false,
    ))]
    pub fn compute_biome_attributes(
        &self,
        py: Python<'_>,
        face_texture_keys: Vec<String>,
        biome_name: &str,
        multi_biomes: Option<Vec<(String, f32)>>,
        custom_temp: Option<f32>,
        custom_humidity: Option<f32>,
        custom_grass: Option<[f32; 4]>,
        custom_foliage: Option<[f32; 4]>,
        custom_dry_foliage: Option<[f32; 4]>,
        custom_water: Option<[f32; 4]>,
        has_custom_grass: bool,
        has_custom_foliage: bool,
        has_custom_dry_foliage: bool,
    ) -> PyResult<PyObject> {
        let res = py.allow_threads(|| {
            if biome_name.eq_ignore_ascii_case("custom")
                || custom_temp.is_some()
                || custom_humidity.is_some()
                || custom_grass.is_some()
                || custom_foliage.is_some()
                || custom_water.is_some()
            {
                let settings = mtk_material::CustomBiomeSettings {
                    temperature: custom_temp.unwrap_or(0.8),
                    humidity: custom_humidity.unwrap_or(0.4),
                    grass_color: custom_grass,
                    foliage_color: custom_foliage,
                    dry_foliage_color: custom_dry_foliage,
                    water_color: custom_water,
                    has_custom_grass,
                    has_custom_foliage,
                    has_custom_dry_foliage,
                };
                mtk_material::compute_mesh_biome_attributes_custom(
                    &face_texture_keys,
                    &settings,
                    &self.inner,
                )
            } else {
                mtk_material::compute_mesh_biome_attributes(
                    &face_texture_keys,
                    biome_name,
                    multi_biomes.as_deref(),
                    &self.inner,
                )
            }
        });

        let dict = PyDict::new(py);
        dict.set_item("packed_tint_data", res.packed_tint_data)?;
        dict.set_item("tint_colors", res.tint_colors)?;
        dict.set_item("colormap_uvs", res.colormap_uvs)?;
        Ok(dict.into())
    }
}
