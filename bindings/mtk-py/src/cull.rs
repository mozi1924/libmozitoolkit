//! # `mtk-py` Face Culling Engine Binding
//!
//! Exposes `FaceCuller` and occlusion rules to Python.

use pyo3::prelude::*;

use mtk_cull::types::{GlassCullMode, LeavesCullMode};
use mtk_cull::FaceCuller;

/// Python wrapper for `FaceCuller` occlusion and geometry optimization engine.
#[pyclass(name = "FaceCuller")]
#[derive(Debug, Clone)]
pub struct PyFaceCuller {
    pub(crate) inner: FaceCuller,
}

#[pymethods]
impl PyFaceCuller {
    #[new]
    #[pyo3(signature = (leaves_cull_mode=0, glass_cull_mode=0))]
    pub fn new(leaves_cull_mode: u8, glass_cull_mode: u8) -> PyResult<Self> {
        let leaves = match leaves_cull_mode {
            0 => LeavesCullMode::SingleFace,
            1 => LeavesCullMode::Fancy,
            2 => LeavesCullMode::Fast,
            3 => LeavesCullMode::None,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid leaves_cull_mode (0: SingleFace, 1: Fancy, 2: Fast, 3: None)",
                ))
            }
        };

        let glass = match glass_cull_mode {
            0 => GlassCullMode::Group,
            1 => GlassCullMode::SameBlock,
            2 => GlassCullMode::None,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid glass_cull_mode (0: Group, 1: SameBlock, 2: None)",
                ))
            }
        };

        Ok(Self {
            inner: FaceCuller::new(leaves, glass),
        })
    }

    /// Clears the internal block culling metadata cache.
    pub fn clear_cache(&self) {
        self.inner.clear_cache();
    }

    /// Number of cached blockstate occlusion metadata entries.
    pub fn cache_len(&self) -> usize {
        self.inner.cache_len()
    }

    fn __repr__(&self) -> String {
        format!(
            "<FaceCuller cached_entries={}>",
            self.inner.cache_len()
        )
    }
}

/// Performs spatial-hashing based face culling on a `PyMeshData` buffer.
///
/// Removes interior touching faces with opposite normals and duplicate overlapping faces.
#[pyfunction]
#[pyo3(signature = (mesh, tolerance=1e-3, cull_coplanar_opposite=true, cull_duplicates=true))]
pub fn cull_mesh_faces<'py>(
    py: Python<'py>,
    mesh: &crate::mesh::PyMeshData,
    tolerance: f32,
    cull_coplanar_opposite: bool,
    cull_duplicates: bool,
) -> PyResult<(crate::mesh::PyMeshData, Bound<'py, pyo3::types::PyDict>)> {
    let cfg = mtk_cull::MeshCullConfig {
        tolerance,
        cull_coplanar_opposite,
        cull_duplicates,
    };

    let result = mtk_cull::cull_mesh_faces(&mesh.inner, &cfg);

    let stats_dict = pyo3::types::PyDict::new(py);
    stats_dict.set_item("initial_faces", result.initial_faces)?;
    stats_dict.set_item("culled_faces", result.culled_faces)?;
    stats_dict.set_item("remaining_faces", result.remaining_faces)?;

    Ok((crate::mesh::PyMeshData { inner: result.mesh }, stats_dict))
}

