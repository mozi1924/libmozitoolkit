use std::collections::HashMap;

use mtk_material::GridAtlasSpec;
use pyo3::prelude::*;

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
}
