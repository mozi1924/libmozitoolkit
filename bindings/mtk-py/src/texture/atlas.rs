//! Atlas Builder and BakedAtlas Python bindings.

//! # `mtk-py` Atlas Builder and Texture Baking Binding
//!
//! Exposes Atlas generation, sprite stitching, and UV mapping tables to Python.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyMemoryView};

use mtk_resource::AtlasCategory;
use mtk_texture::{AtlasBuilder, AtlasBuilderConfig, BakedAtlas};

use crate::resource::PyResourcePackStack;

/// Complete baked atlas sheets and UV address map.
#[pyclass(name = "BakedAtlas")]
#[derive(Debug, Clone)]
pub struct PyBakedAtlas {
    pub(crate) inner: BakedAtlas,
}

#[pymethods]
impl PyBakedAtlas {
    /// Create a BakedAtlas wrapper from an atlas mapping JSON string.
    #[staticmethod]
    pub fn from_mapping_json(json_str: &str) -> PyResult<Self> {
        let address_map = mtk_texture::AtlasAddressMap::from_json(json_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner: BakedAtlas {
                chunks: Vec::new(),
                address_map,
            },
        })
    }

    /// Returns chunk metadata dictionary by chunk ID:
    /// `{chunk_id, category, is_animated, width, height, has_normal, has_specular, has_overlay, file_stem, category_chunk_index}`.
    pub fn get_chunk_info<'py>(
        &self,
        py: Python<'py>,
        chunk_id: u16,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        if let Some(cm) = self.inner.get_chunk_meta(chunk_id) {
            let dict = PyDict::new(py);
            dict.set_item("chunk_id", cm.chunk_id)?;
            dict.set_item("category", &cm.category)?;
            dict.set_item("is_animated", cm.is_animated)?;
            dict.set_item("category_chunk_index", cm.category_chunk_index)?;
            dict.set_item("width", cm.width)?;
            dict.set_item("height", cm.height)?;
            dict.set_item("has_normal", cm.has_normal)?;
            dict.set_item("has_specular", cm.has_specular)?;
            dict.set_item("has_overlay", cm.has_overlay)?;
            dict.set_item("file_stem", cm.file_stem())?;
            Ok(Some(dict))
        } else {
            Ok(None)
        }
    }

    /// Returns a list of dictionaries for all chunks in the atlas.
    pub fn get_all_chunks_info<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, pyo3::types::PyList>> {
        let list = pyo3::types::PyList::empty(py);
        for cm in self.inner.address_map.get_chunks() {
            let dict = PyDict::new(py);
            dict.set_item("chunk_id", cm.chunk_id)?;
            dict.set_item("category", &cm.category)?;
            dict.set_item("is_animated", cm.is_animated)?;
            dict.set_item("category_chunk_index", cm.category_chunk_index)?;
            dict.set_item("width", cm.width)?;
            dict.set_item("height", cm.height)?;
            dict.set_item("has_normal", cm.has_normal)?;
            dict.set_item("has_specular", cm.has_specular)?;
            dict.set_item("has_overlay", cm.has_overlay)?;
            dict.set_item("file_stem", cm.file_stem())?;
            list.append(dict)?;
        }
        Ok(list)
    }

    /// Returns chunk metadata tuple: `(width, height, is_animated, category, file_stem)`.
    pub fn get_chunk_meta(&self, index: usize) -> PyResult<(u32, u32, bool, String, String)> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            Ok((
                chunk.width,
                chunk.height,
                chunk.is_animated,
                chunk.category.clone(),
                chunk.file_stem(),
            ))
        } else if let Some(chunk) = self.inner.address_map.chunks.get(index) {
            Ok((
                chunk.width,
                chunk.height,
                chunk.is_animated,
                chunk.category.clone(),
                chunk.file_stem(),
            ))
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "Chunk index out of bounds",
            ))
        }
    }

    /// Read-only memoryview of Albedo normalized float32 pixels (`width * height * 4`).
    /// `flip_v`: If True (default True for Blender convention), flips rows vertically.
    #[pyo3(signature = (index, flip_v=true))]
    pub fn get_chunk_albedo_f32<'py>(
        &self,
        py: Python<'py>,
        index: usize,
        flip_v: bool,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            let f32_buf = chunk.albedo.to_f32_buffer(flip_v);
            crate::buffer::create_owned_f32_memoryview(py, f32_buf)
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "Chunk index out of bounds",
            ))
        }
    }

    /// Read-only memoryview of Normal companion normalized float32 pixels (if present).
    #[pyo3(signature = (index, flip_v=true))]
    pub fn get_chunk_normal_f32<'py>(
        &self,
        py: Python<'py>,
        index: usize,
        flip_v: bool,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref n) = chunk.normal {
                let f32_buf = n.to_f32_buffer(flip_v);
                Ok(Some(crate::buffer::create_owned_f32_memoryview(
                    py, f32_buf,
                )?))
            } else {
                Ok(None)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "Chunk index out of bounds",
            ))
        }
    }

    /// Read-only memoryview of Specular companion normalized float32 pixels (if present).
    #[pyo3(signature = (index, flip_v=true))]
    pub fn get_chunk_specular_f32<'py>(
        &self,
        py: Python<'py>,
        index: usize,
        flip_v: bool,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref s) = chunk.specular {
                let f32_buf = s.to_f32_buffer(flip_v);
                Ok(Some(crate::buffer::create_owned_f32_memoryview(
                    py, f32_buf,
                )?))
            } else {
                Ok(None)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "Chunk index out of bounds",
            ))
        }
    }

    /// Read-only memoryview of Overlay companion normalized float32 pixels (if present).
    #[pyo3(signature = (index, flip_v=true))]
    pub fn get_chunk_overlay_f32<'py>(
        &self,
        py: Python<'py>,
        index: usize,
        flip_v: bool,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref o) = chunk.overlay {
                let f32_buf = o.to_f32_buffer(flip_v);
                Ok(Some(crate::buffer::create_owned_f32_memoryview(
                    py, f32_buf,
                )?))
            } else {
                Ok(None)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "Chunk index out of bounds",
            ))
        }
    }

    /// Serializes the atlas address map into a JSON string.
    pub fn to_mapping_json(&self) -> PyResult<String> {
        self.inner
            .address_map
            .to_json()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "<BakedAtlas chunks={} sprites={}>",
            self.inner.chunks.len(),
            self.inner.address_map.sprites.len()
        )
    }
}

/// Atlas generator for Minecraft resource packs.
#[pyclass(name = "AtlasBuilder")]
#[derive(Debug, Clone)]
pub struct PyAtlasBuilder {
    config: AtlasBuilderConfig,
}

#[pymethods]
impl PyAtlasBuilder {
    #[new]
    #[pyo3(signature = (max_width=4096, max_height=4096, mip_level=0, padding=0))]
    pub fn new(max_width: u32, max_height: u32, mip_level: u32, padding: u32) -> Self {
        Self {
            config: AtlasBuilderConfig {
                max_width,
                max_height,
                mip_level,
                padding,
            },
        }
    }

    /// Bakes an atlas category (e.g. `"blocks"`, `"items"`, `"particles"`) from the resource stack.
    #[pyo3(signature = (stack, category="blocks"))]
    pub fn build(
        &self,
        py: Python<'_>,
        stack: &PyResourcePackStack,
        category: &str,
    ) -> PyResult<PyBakedAtlas> {
        let cat = match category {
            "blocks" => AtlasCategory::Blocks,
            "items" => AtlasCategory::Items,
            "particles" => AtlasCategory::Particles,
            "paintings" => AtlasCategory::Paintings,
            "banner_patterns" => AtlasCategory::BannerPatterns,
            "shield_patterns" => AtlasCategory::ShieldPatterns,
            "shulker_boxes" => AtlasCategory::ShulkerBoxes,
            "chests" => AtlasCategory::Chests,
            "armor_trims" => AtlasCategory::ArmorTrims,
            _ => AtlasCategory::Blocks,
        };

        let builder = AtlasBuilder::new(self.config.clone());
        let baked = py
            .allow_threads(|| builder.build_category(&stack.inner, &cat))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyBakedAtlas { inner: baked })
    }
}
