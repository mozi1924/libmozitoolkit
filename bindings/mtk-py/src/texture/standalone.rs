//! Standalone PBR and material asset library Python bindings.

use crate::resource::PyResourcePackStack;
use mtk_texture::{StandaloneBuilder, StandaloneConfig};
use pyo3::prelude::*;

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
        py: Python<'_>,
        stack: &PyResourcePackStack,
        output_dir: &str,
    ) -> PyResult<PyStandaloneResult> {
        let builder = StandaloneBuilder::new(self.config.clone());
        let res = py
            .allow_threads(|| builder.build_to_dir(&stack.inner, output_dir))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyStandaloneResult {
            mapping_path: res.mapping_path.to_string_lossy().to_string(),
            output_dir: res.output_dir.to_string_lossy().to_string(),
            texture_count: res.texture_count,
            format_version: res.format_version,
        })
    }
}

#[pyfunction]
#[pyo3(signature = (u, v, width, height, pixels, invert_y=false))]
pub fn sample_uv_alpha_f32(
    u: f32,
    v: f32,
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    invert_y: bool,
) -> f32 {
    mtk_texture::sample_alpha_f32(width, height, &pixels, u, v, invert_y)
}

#[pyfunction]
#[pyo3(signature = (face_uvs, width, height, pixels, mode="CENTER", threshold=0.01, invert_y=false))]
pub fn is_face_transparent_f32(
    face_uvs: Vec<(f32, f32)>,
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    mode: &str,
    threshold: f32,
    invert_y: bool,
) -> bool {
    let vecs: Vec<glam::Vec2> = face_uvs
        .iter()
        .map(|(u, v)| glam::Vec2::new(*u, *v))
        .collect();
    let sample_mode = mtk_texture::SampleMode::from_str(mode);
    mtk_texture::is_face_transparent_f32(
        &vecs,
        width,
        height,
        &pixels,
        sample_mode,
        threshold,
        invert_y,
    )
}

#[pyfunction]
#[pyo3(signature = (faces_uvs, width, height, pixels, mode="CENTER", threshold=0.01, invert_y=false))]
pub fn batch_analyze_transparent_faces_f32(
    py: Python<'_>,
    faces_uvs: Vec<Vec<(f32, f32)>>,
    width: u32,
    height: u32,
    pixels: Vec<f32>,
    mode: &str,
    threshold: f32,
    invert_y: bool,
) -> Vec<bool> {
    let faces_vecs: Vec<Vec<glam::Vec2>> = faces_uvs
        .into_iter()
        .map(|uvs| uvs.iter().map(|(u, v)| glam::Vec2::new(*u, *v)).collect())
        .collect();
    let sample_mode = mtk_texture::SampleMode::from_str(mode);
    py.allow_threads(|| {
        mtk_texture::batch_analyze_transparent_faces_f32(
            &faces_vecs,
            width,
            height,
            &pixels,
            sample_mode,
            threshold,
            invert_y,
        )
    })
}

#[pyfunction]
#[pyo3(signature = (faces_uvs, width, height, pixels, mode="CENTER", threshold=0.01, invert_y=false))]
pub fn batch_analyze_transparent_faces_u8(
    py: Python<'_>,
    faces_uvs: Vec<Vec<(f32, f32)>>,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    mode: &str,
    threshold: f32,
    invert_y: bool,
) -> Vec<bool> {
    let faces_vecs: Vec<Vec<glam::Vec2>> = faces_uvs
        .into_iter()
        .map(|uvs| uvs.iter().map(|(u, v)| glam::Vec2::new(*u, *v)).collect())
        .collect();
    let sample_mode = mtk_texture::SampleMode::from_str(mode);
    py.allow_threads(|| {
        mtk_texture::batch_analyze_transparent_faces_u8(
            &faces_vecs,
            width,
            height,
            &pixels,
            sample_mode,
            threshold,
            invert_y,
        )
    })
}
