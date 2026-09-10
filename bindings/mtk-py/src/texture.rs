//! # `mtk-py` Atlas Builder and Texture Baking Binding
//!
//! Exposes Atlas generation, sprite stitching, and UV mapping tables to Python.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyMemoryView};

use mtk_resource::{AtlasCategory, ResourceLocation};
use mtk_texture::{
    AtlasBuilder, AtlasBuilderConfig, BakedAtlas, StandaloneBuilder, StandaloneConfig,
};

use crate::resource::PyResourcePackStack;

/// Complete baked atlas sheets and UV address map.
#[pyclass(name = "BakedAtlas")]
#[derive(Debug, Clone)]
pub struct PyBakedAtlas {
    pub(crate) inner: BakedAtlas,
}

#[pymethods]
impl PyBakedAtlas {
    /// Number of baked texture sheets / chunks.
    pub fn get_chunk_count(&self) -> usize {
        self.inner.chunks.len()
    }

    /// Returns chunk metadata: `(width, height, is_animated, category, file_stem)`.
    pub fn get_chunk_meta(&self, index: usize) -> PyResult<(u32, u32, bool, String, String)> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            Ok((
                chunk.width,
                chunk.height,
                chunk.is_animated,
                chunk.category.clone(),
                chunk.file_stem(),
            ))
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Read-only memoryview of Albedo RGBA pixel bytes (`width * height * 4`).
    pub fn get_chunk_albedo_memoryview<'py>(
        &self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            let byte_slice = &chunk.albedo.pixels;
            let bytes = PyBytes::new(py, byte_slice);
            PyMemoryView::from(&bytes)
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Read-only memoryview of Normal companion RGBA pixel bytes (if present).
    pub fn get_chunk_normal_memoryview<'py>(
        &self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref n) = chunk.normal {
                let bytes = PyBytes::new(py, &n.pixels);
                Ok(Some(PyMemoryView::from(&bytes)?))
            } else {
                Ok(None)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Read-only memoryview of Specular companion RGBA pixel bytes (if present).
    pub fn get_chunk_specular_memoryview<'py>(
        &self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref s) = chunk.specular {
                let bytes = PyBytes::new(py, &s.pixels);
                Ok(Some(PyMemoryView::from(&bytes)?))
            } else {
                Ok(None)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Saves the chunk's Albedo atlas texture as a PNG file to the given path.
    pub fn save_chunk_albedo_png(&self, index: usize, path: &str) -> PyResult<()> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            let png_bytes = chunk
                .albedo
                .to_png_bytes()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            std::fs::write(path, png_bytes)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            Ok(())
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Saves the chunk's Normal companion atlas texture as a PNG file (if present).
    pub fn save_chunk_normal_png(&self, index: usize, path: &str) -> PyResult<bool> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref n) = chunk.normal {
                let png_bytes = n
                    .to_png_bytes()
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                std::fs::write(path, png_bytes)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Saves the chunk's Specular companion atlas texture as a PNG file (if present).
    pub fn save_chunk_specular_png(&self, index: usize, path: &str) -> PyResult<bool> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            if let Some(ref s) = chunk.specular {
                let png_bytes = s
                    .to_png_bytes()
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                std::fs::write(path, png_bytes)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Returns encoded PNG bytes of the chunk's Albedo atlas.
    pub fn get_chunk_albedo_png_bytes<'py>(
        &self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        if let Some(chunk) = self.inner.chunks.get(index) {
            let png_bytes = chunk
                .albedo
                .to_png_bytes()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            Ok(PyBytes::new(py, &png_bytes))
        } else {
            Err(pyo3::exceptions::PyIndexError::new_err("Chunk index out of bounds"))
        }
    }

    /// Queries the baked UV coordinates and metadata for a given texture resource identifier.
    ///
    /// Returns dictionary with keys: `chunk_id`, `uv_bounds`, `frame_0_uv_bounds`, `is_animated`, `frame_count`.
    pub fn lookup_sprite<'py>(
        &self,
        py: Python<'py>,
        location: &str,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        let loc = ResourceLocation::parse(location)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        if let Some(meta) = self.inner.address_map.lookup(&loc) {
            let dict = PyDict::new(py);
            dict.set_item("chunk_id", meta.chunk_id)?;
            dict.set_item("uv_bounds", meta.uv_bounds)?;
            dict.set_item("frame_0_uv_bounds", meta.frame_0_uv_bounds)?;
            dict.set_item("is_animated", meta.is_animated)?;
            dict.set_item("frame_count", meta.frame_count)?;
            dict.set_item("has_normal", meta.has_normal)?;
            dict.set_item("has_specular", meta.has_specular)?;
            Ok(Some(dict))
        } else {
            Ok(None)
        }
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
        let baked = builder
            .build_category(&stack.inner, &cat)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyBakedAtlas { inner: baked })
    }
}

/// Standalone Material Asset Library precompiler result.
#[pyclass(name = "StandaloneResult")]
#[derive(Debug, Clone)]
pub struct PyStandaloneResult {
    #[pyo3(get)]
    pub mapping_path: String,
    #[pyo3(get)]
    pub output_dir: String,
    #[pyo3(get)]
    pub texture_count: usize,
    #[pyo3(get)]
    pub format_version: u32,
}

#[pymethods]
impl PyStandaloneResult {
    fn __repr__(&self) -> String {
        format!(
            "<StandaloneResult textures={} dir='{}'>",
            self.texture_count, self.output_dir
        )
    }
}

/// Standalone Material Asset Library Generator for resource pack stacks.
#[pyclass(name = "StandaloneBuilder")]
#[derive(Debug, Clone)]
pub struct PyStandaloneBuilder {
    config: StandaloneConfig,
}

#[pymethods]
impl PyStandaloneBuilder {
    #[new]
    #[pyo3(signature = (stack_hash=None, filter_prefix=None))]
    pub fn new(stack_hash: Option<String>, filter_prefix: Option<String>) -> Self {
        Self {
            config: StandaloneConfig {
                stack_hash,
                filter_prefix,
            },
        }
    }

    /// Precompiles all standalone textures from the given pack stack into `output_dir`.
    #[pyo3(signature = (stack, output_dir))]
    pub fn build(
        &self,
        stack: &PyResourcePackStack,
        output_dir: &str,
    ) -> PyResult<PyStandaloneResult> {
        let builder = StandaloneBuilder::new(self.config.clone());
        let res = builder
            .build_to_dir(&stack.inner, output_dir)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyStandaloneResult {
            mapping_path: res.mapping_path.to_string_lossy().to_string(),
            output_dir: res.output_dir.to_string_lossy().to_string(),
            texture_count: res.texture_count,
            format_version: res.format_version,
        })
    }
}

