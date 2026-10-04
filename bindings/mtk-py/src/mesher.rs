use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use mtk_cull::FaceCuller;
use mtk_voxel::delta_mesher::DeltaMesher;
use mtk_voxel::mesher::SectionMesher;
use mtk_voxel::types::MesherConfig;

use crate::cull::PyFaceCuller;
use crate::mesh::PyMeshData;
use crate::model::PyBakedModelDatabase;
use crate::voxel::{PyMesherConfig, PyVoxelStorage};

/// Python wrapper for Section and World Mesher generator.
#[pyclass(name = "SectionMesher")]
pub struct PySectionMesher;

#[pymethods]
impl PySectionMesher {
    /// Meshes the entire `VoxelStorage` and returns a single merged `MeshData`.
    #[staticmethod]
    #[pyo3(signature = (storage, config=None, culler=None, model_db=None))]
    pub fn mesh_world(
        py: Python<'_>,
        storage: &PyVoxelStorage,
        config: Option<&PyMesherConfig>,
        culler: Option<&PyFaceCuller>,
        model_db: Option<&PyBakedModelDatabase>,
    ) -> PyResult<PyMeshData> {
        let default_config = MesherConfig::default();
        let mut cfg = config.map(|c| c.inner.clone()).unwrap_or(default_config);
        if cfg.origin_centered && cfg.selection_bounds.is_none() {
            let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = storage.inner.get_bounds();
            if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                cfg.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
            }
        }
        let num_threads = config.and_then(|c| c.num_threads);

        let default_culler = FaceCuller::default();
        let cul = culler.map(|c| &c.inner).unwrap_or(&default_culler);

        let non_empty = storage.inner.get_all_non_empty_sections();
        if non_empty.is_empty() {
            return Ok(PyMeshData::new());
        }

        let model_db_opt = model_db.map(|db| db.inner.clone());
        let model_lookup = move |state: &str| -> mtk_voxel::mesher::ModelSource {
            if let Some(ref db) = model_db_opt {
                if let Some(group) = db.get_variant_group(state) {
                    return mtk_voxel::mesher::ModelSource::Variant(Arc::new(group.clone()));
                }
                if let Some(model) = db.get(state) {
                    return mtk_voxel::mesher::ModelSource::Single(Arc::new(model.clone()));
                }
            }
            mtk_voxel::mesher::ModelSource::None
        };

        let merged_mesh = py.allow_threads(|| -> Result<mtk_core::mesh::MeshData, String> {
            let padded_sections = mtk_core::constants::concurrency::execute_parallel(num_threads, || {
                #[cfg(feature = "parallel")]
                {
                    use rayon::prelude::*;
                    non_empty
                        .par_iter()
                        .map(|&coord| storage.inner.get_section_padded_array(coord))
                        .collect::<Vec<_>>()
                }
                #[cfg(not(feature = "parallel"))]
                {
                    non_empty
                        .iter()
                        .map(|&coord| storage.inner.get_section_padded_array(coord))
                        .collect::<Vec<_>>()
                }
            })?;

            let results = SectionMesher::mesh_sections_parallel_with_source(
                &padded_sections,
                cul,
                model_lookup,
                &cfg,
                num_threads,
            )
            .map_err(|e| e.to_string())?;

            let section_meshes: Vec<_> = results.iter().map(|(_, m)| m).collect();
            let mut merged = mtk_core::mesh::MeshData::merge_all_refs(&section_meshes);

            if cfg.weld_vertices {
                merged.weld_spatial_vertices(1e-4);
            }

            Ok(merged)
        })
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))?;

        Ok(PyMeshData { inner: merged_mesh })
    }

    /// Meshes all non-empty sections and returns a dictionary mapping `(sx, sy, sz)` to `MeshData`.
    #[staticmethod]
    #[pyo3(signature = (storage, config=None, culler=None, model_db=None))]
    pub fn mesh_sections_split<'py>(
        py: Python<'py>,
        storage: &PyVoxelStorage,
        config: Option<&PyMesherConfig>,
        culler: Option<&PyFaceCuller>,
        model_db: Option<&PyBakedModelDatabase>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let default_config = MesherConfig::default();
        let mut cfg = config.map(|c| c.inner.clone()).unwrap_or(default_config);
        if cfg.origin_centered && cfg.selection_bounds.is_none() {
            let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = storage.inner.get_bounds();
            if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                cfg.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
            }
        }
        let num_threads = config.and_then(|c| c.num_threads);

        let default_culler = FaceCuller::default();
        let cul = culler.map(|c| &c.inner).unwrap_or(&default_culler);

        let non_empty = storage.inner.get_all_non_empty_sections();
        let dict = PyDict::new(py);

        if non_empty.is_empty() {
            return Ok(dict);
        }

        let model_db_opt = model_db.map(|db| db.inner.clone());
        let model_lookup = move |state: &str| -> mtk_voxel::mesher::ModelSource {
            if let Some(ref db) = model_db_opt {
                if let Some(group) = db.get_variant_group(state) {
                    return mtk_voxel::mesher::ModelSource::Variant(Arc::new(group.clone()));
                }
                if let Some(model) = db.get(state) {
                    return mtk_voxel::mesher::ModelSource::Single(Arc::new(model.clone()));
                }
            }
            mtk_voxel::mesher::ModelSource::None
        };

        let results = py.allow_threads(|| -> Result<Vec<(glam::IVec3, mtk_core::mesh::MeshData)>, String> {
            let padded_sections = mtk_core::constants::concurrency::execute_parallel(num_threads, || {
                #[cfg(feature = "parallel")]
                {
                    use rayon::prelude::*;
                    non_empty
                        .par_iter()
                        .map(|&coord| storage.inner.get_section_padded_array(coord))
                        .collect::<Vec<_>>()
                }
                #[cfg(not(feature = "parallel"))]
                {
                    non_empty
                        .iter()
                        .map(|&coord| storage.inner.get_section_padded_array(coord))
                        .collect::<Vec<_>>()
                }
            })?;

            SectionMesher::mesh_sections_parallel_with_source(
                &padded_sections,
                cul,
                model_lookup,
                &cfg,
                num_threads,
            )
            .map_err(|e| e.to_string())
        })
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))?;

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
    #[pyo3(signature = (storage, config=None, culler=None, model_db=None))]
    pub fn rebuild_dirty_sections<'py>(
        py: Python<'py>,
        storage: &mut PyVoxelStorage,
        config: Option<&PyMesherConfig>,
        culler: Option<&PyFaceCuller>,
        model_db: Option<&PyBakedModelDatabase>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let default_config = MesherConfig::default();
        let mut cfg = config.map(|c| c.inner.clone()).unwrap_or(default_config);
        if cfg.origin_centered && cfg.selection_bounds.is_none() {
            let (min_x, min_y, min_z, sz_x, sz_y, sz_z) = storage.inner.get_bounds();
            if sz_x > 0 && sz_y > 0 && sz_z > 0 {
                cfg.selection_bounds = Some(([min_x, min_y, min_z], [sz_x, sz_y, sz_z]));
            }
        }

        let default_culler = FaceCuller::default();
        let cul = culler.map(|c| &c.inner).unwrap_or(&default_culler);

        let model_db_opt = model_db.map(|db| db.inner.clone());
        let model_lookup = move |state: &str| -> mtk_voxel::mesher::ModelSource {
            if let Some(ref db) = model_db_opt {
                if let Some(group) = db.get_variant_group(state) {
                    return mtk_voxel::mesher::ModelSource::Variant(Arc::new(group.clone()));
                }
                if let Some(model) = db.get(state) {
                    return mtk_voxel::mesher::ModelSource::Single(Arc::new(model.clone()));
                }
            }
            mtk_voxel::mesher::ModelSource::None
        };

        let results = py.allow_threads(|| {
            DeltaMesher::rebuild_dirty_sections_with_source(
                &mut storage.inner,
                cul,
                model_lookup,
                &cfg,
            )
        });

        let dict = PyDict::new(py);
        for (coord, mesh) in results {
            let key = (coord.x, coord.y, coord.z);
            let py_mesh = PyMeshData { inner: mesh };
            dict.set_item(key, py_mesh)?;
        }

        Ok(dict)
    }
}
