//! # `mtk-py` Minecraft Save Loader Bindings
//!
//! Exposes `mtk-save` level metadata, spatial chunk querying, and direct
//! world meshing pipelines to Python.

use std::path::Path;
use glam::IVec3;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use mtk_save::{LevelData, SaveLoader};
use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::VoxelWorld;

use crate::cull::PyFaceCuller;
use crate::mesh::PyMeshData;
use crate::model::PyBakedModelDatabase;
use crate::voxel::{PyMesherConfig, PyVoxelStorage};

/// Python wrapper for Minecraft world metadata from `level.dat`.
#[pyclass(name = "LevelData")]
#[derive(Debug, Clone)]
pub struct PyLevelData {
    pub(crate) inner: LevelData,
    pub(crate) dimensions: Vec<String>,
}

#[pymethods]
impl PyLevelData {
    #[getter]
    pub fn level_name(&self) -> &str {
        &self.inner.level_name
    }

    #[getter]
    pub fn version_name(&self) -> &str {
        &self.inner.version_name
    }

    #[getter]
    pub fn data_version(&self) -> i32 {
        self.inner.data_version
    }

    #[getter]
    pub fn spawn_x(&self) -> i32 {
        self.inner.spawn.x
    }

    #[getter]
    pub fn spawn_y(&self) -> i32 {
        self.inner.spawn.y
    }

    #[getter]
    pub fn spawn_z(&self) -> i32 {
        self.inner.spawn.z
    }

    #[getter]
    pub fn spawn(&self) -> (i32, i32, i32) {
        (self.inner.spawn.x, self.inner.spawn.y, self.inner.spawn.z)
    }

    #[getter]
    pub fn time(&self) -> i64 {
        self.inner.time
    }

    #[getter]
    pub fn day_time(&self) -> i64 {
        self.inner.day_time
    }

    #[getter]
    pub fn hardcore(&self) -> bool {
        self.inner.hardcore
    }

    #[getter]
    pub fn game_type(&self) -> i32 {
        self.inner.game_type
    }

    #[getter]
    pub fn dimensions(&self) -> Vec<String> {
        self.dimensions.clone()
    }

    /// Converts metadata into a Python dictionary.
    pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("level_name", &self.inner.level_name)?;
        dict.set_item("version_name", &self.inner.version_name)?;
        dict.set_item("data_version", self.inner.data_version)?;
        dict.set_item("spawn", (self.inner.spawn.x, self.inner.spawn.y, self.inner.spawn.z))?;
        dict.set_item("time", self.inner.time)?;
        dict.set_item("day_time", self.inner.day_time)?;
        dict.set_item("hardcore", self.inner.hardcore)?;
        dict.set_item("game_type", self.inner.game_type)?;
        dict.set_item("dimensions", self.dimensions.clone())?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<LevelData name=\"{}\" ver=\"{}\" dv={} spawn=({}, {}, {}) dims={:?}>",
            self.inner.level_name,
            self.inner.version_name,
            self.inner.data_version,
            self.inner.spawn.x,
            self.inner.spawn.y,
            self.inner.spawn.z,
            self.dimensions
        )
    }
}

/// Inspects a Minecraft save folder, reading `level.dat` and discovering available dimensions.
#[pyfunction]
pub fn inspect_minecraft_save(world_dir: &str) -> PyResult<PyLevelData> {
    let path = Path::new(world_dir);
    let level_data = SaveLoader::read_level_data(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    let dimensions = SaveLoader::discover_dimensions(path);

    Ok(PyLevelData {
        inner: level_data,
        dimensions,
    })
}

/// Loads a bounded 3D selection box from a Minecraft save directly into a `VoxelStorage`.
#[pyfunction]
#[pyo3(signature = (world_dir, dimension="overworld", min_block=(-64, -64, -64), max_block=(64, 320, 64)))]
pub fn load_minecraft_save_storage(
    world_dir: &str,
    dimension: &str,
    min_block: (i32, i32, i32),
    max_block: (i32, i32, i32),
) -> PyResult<(PyVoxelStorage, PyLevelData)> {
    let path = Path::new(world_dir);
    let level_data = SaveLoader::read_level_data(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    let dimensions = SaveLoader::discover_dimensions(path);

    let min_coord = IVec3::new(min_block.0, min_block.1, min_block.2);
    let max_coord = IVec3::new(max_block.0, max_block.1, max_block.2);

    let mut storage = VoxelStorage::new();
    SaveLoader::load_box_into_storage(path, dimension, min_coord, max_coord, &mut storage)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok((
        PyVoxelStorage { inner: storage },
        PyLevelData {
            inner: level_data,
            dimensions,
        },
    ))
}

/// End-to-end pipeline: Loads bounded 3D world selection from save, feeds into `VoxelWorld`,
/// and meshes into `PyMeshData` with Atlas UVs and Ambient Occlusion.
#[pyfunction]
#[pyo3(signature = (world_dir, dimension="overworld", min_block=(-64, -64, -64), max_block=(64, 320, 64), config=None, culler=None, model_db=None, num_threads=None))]
pub fn load_and_mesh_minecraft_save(
    py: Python<'_>,
    world_dir: &str,
    dimension: &str,
    min_block: (i32, i32, i32),
    max_block: (i32, i32, i32),
    config: Option<&PyMesherConfig>,
    culler: Option<&PyFaceCuller>,
    model_db: Option<&PyBakedModelDatabase>,
    num_threads: Option<usize>,
) -> PyResult<(PyMeshData, PyLevelData, PyVoxelStorage)> {
    let path = Path::new(world_dir);
    let level_data = SaveLoader::read_level_data(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    let dimensions = SaveLoader::discover_dimensions(path);

    let min_coord = IVec3::new(min_block.0, min_block.1, min_block.2);
    let max_coord = IVec3::new(max_block.0, max_block.1, max_block.2);

    let mut storage = VoxelStorage::new();
    SaveLoader::load_box_into_storage(path, dimension, min_coord, max_coord, &mut storage)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let cfg = config.map(|c| c.inner.clone());
    let cul = culler.map(|c| c.inner.clone());
    let mdb = model_db.map(|db| db.inner.clone());
    let threads = num_threads.or_else(|| config.and_then(|c| c.num_threads));

    let mut world = VoxelWorld::from_storage_with_threads(
        storage.clone(),
        cfg,
        cul,
        mdb,
        true,
        threads,
    );

    let mesh = py
        .allow_threads(|| world.rebuild_all().map(|m| m.clone()))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok((
        PyMeshData { inner: mesh },
        PyLevelData {
            inner: level_data,
            dimensions,
        },
        PyVoxelStorage { inner: storage },
    ))
}
