//! # `mtk-py` Mesh Generator and Delta Mesher Binding
//!
//! Exposes parallel Section meshing (`SectionMesher`) and incremental dirty section
//! rebuilding (`DeltaMesher`) to Python.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use mtk_cull::FaceCuller;
use mtk_voxel::delta_mesher::DeltaMesher;
use mtk_voxel::mesher::SectionMesher;
use mtk_voxel::types::MesherConfig;

use crate::cull::PyFaceCuller;
use crate::mesh::PyMeshData;
use crate::voxel::{PyMesherConfig, PyVoxelStorage};

/// Python wrapper for Section and World Mesher generator.
#[pyclass(name = "SectionMesher")]
pub struct PySectionMesher;

#[pymethods]
impl PySectionMesher {
    /// Meshes the entire `VoxelStorage` and returns a single merged `MeshData`.
    #[staticmethod]
    #[pyo3(signature = (storage, config=None, culler=None))]
    pub fn mesh_world(
        storage: &PyVoxelStorage,
        config: Option<&PyMesherConfig>,
        culler: Option<&PyFaceCuller>,
    ) -> PyResult<PyMeshData> {
        let default_config = MesherConfig::default();
        let cfg = config.map(|c| &c.inner).unwrap_or(&default_config);
        let num_threads = config.and_then(|c| c.num_threads);

        let default_culler = FaceCuller::default();
        let cul = culler.map(|c| &c.inner).unwrap_or(&default_culler);

        let non_empty = storage.inner.get_all_non_empty_sections();
        if non_empty.is_empty() {
            return Ok(PyMeshData::new());
        }

        let padded_sections: Vec<_> = non_empty
            .into_iter()
            .map(|coord| storage.inner.get_section_padded_array(coord))
            .collect();

        let results = SectionMesher::mesh_sections_parallel(
            &padded_sections,
            cul,
            |_| None,
            cfg,
            num_threads,
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let mut merged = PyMeshData::new();
        for (_coord, mesh) in results {
            merged.inner.append_mesh(&mesh);
        }

        Ok(merged)
    }

    /// Meshes all non-empty sections and returns a dictionary mapping `(sx, sy, sz)` to `MeshData`.
    #[staticmethod]
    #[pyo3(signature = (storage, config=None, culler=None))]
    pub fn mesh_sections_split<'py>(
        py: Python<'py>,
        storage: &PyVoxelStorage,
        config: Option<&PyMesherConfig>,
        culler: Option<&PyFaceCuller>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let default_config = MesherConfig::default();
        let cfg = config.map(|c| &c.inner).unwrap_or(&default_config);
        let num_threads = config.and_then(|c| c.num_threads);

        let default_culler = FaceCuller::default();
        let cul = culler.map(|c| &c.inner).unwrap_or(&default_culler);

        let non_empty = storage.inner.get_all_non_empty_sections();
        let dict = PyDict::new(py);

        if non_empty.is_empty() {
            return Ok(dict);
        }

        let padded_sections: Vec<_> = non_empty
            .into_iter()
            .map(|coord| storage.inner.get_section_padded_array(coord))
            .collect();

        let results = SectionMesher::mesh_sections_parallel(
            &padded_sections,
            cul,
            |_| None,
            cfg,
            num_threads,
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        for (coord, mesh) in results {
            let key = (coord.x, coord.y, coord.z);
            let py_mesh = PyMeshData { inner: mesh };
            dict.set_item(key, py_mesh)?;
        }

        Ok(dict)
    }

    /// Incrementally rebuilds only the modified/dirty sections in `VoxelStorage` and clears their dirty state.
    ///
    /// Returns a dictionary mapping `(sx, sy, sz)` to rebuilt `MeshData`.
    #[staticmethod]
    #[pyo3(signature = (storage, config=None, culler=None))]
    pub fn rebuild_dirty_sections<'py>(
        py: Python<'py>,
        storage: &mut PyVoxelStorage,
        config: Option<&PyMesherConfig>,
        culler: Option<&PyFaceCuller>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let default_config = MesherConfig::default();
        let cfg = config.map(|c| &c.inner).unwrap_or(&default_config);

        let default_culler = FaceCuller::default();
        let cul = culler.map(|c| &c.inner).unwrap_or(&default_culler);

        let results = DeltaMesher::rebuild_dirty_sections(
            &mut storage.inner,
            cul,
            |_| None,
            cfg,
        );

        let dict = PyDict::new(py);
        for (coord, mesh) in results {
            let key = (coord.x, coord.y, coord.z);
            let py_mesh = PyMeshData { inner: mesh };
            dict.set_item(key, py_mesh)?;
        }

        Ok(dict)
    }
}
