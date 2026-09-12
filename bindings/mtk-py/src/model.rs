//! # `mtk-py` Model Baker Binding
//!
//! Exposes universal headless BlockState and Minecraft model baking to Python.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use mtk_model::{BakedModelDatabase, BlockModelJson, BlockState, BlockStateDefinition, ModelBaker};
use crate::mesh::PyMeshData;
use crate::resource::PyResourcePackStack;

/// Prebaked Model Database container.
#[pyclass(name = "BakedModelDatabase")]
#[derive(Default, Clone)]
pub struct PyBakedModelDatabase {
    pub(crate) inner: BakedModelDatabase,
}

#[pymethods]
impl PyBakedModelDatabase {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: BakedModelDatabase::new(),
        }
    }

    /// Number of prebaked models in the database.
    pub fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Checks if a blockstate is contained in the database.
    pub fn contains(&self, state_str: &str) -> bool {
        self.inner.get(state_str).is_some()
    }

    /// Returns list of all canonical blockstate strings in the database.
    pub fn get_states(&self) -> Vec<String> {
        self.inner.keys().cloned().collect()
    }

    /// Retrieves baked mesh geometry and texture list for a given canonical blockstate string.
    #[pyo3(signature = (state_str, clip_hidden=true))]
    pub fn get_mesh(&self, state_str: &str, clip_hidden: bool) -> Option<(PyMeshData, Vec<String>)> {
        self.inner.get(state_str).map(|baked| {
            let (mesh, textures) = baked.to_mesh_with_textures(clip_hidden);
            (PyMeshData { inner: mesh }, textures)
        })
    }

    /// Serializes entire database into compact binary bytes (bincode).
    pub fn to_bincode_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self
            .inner
            .to_bincode()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Deserializes database from compact binary bytes (bincode).
    #[staticmethod]
    pub fn from_bincode_bytes(bytes: &[u8]) -> PyResult<Self> {
        let inner = BakedModelDatabase::from_bincode(bytes)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }
}

/// Headless Minecraft Model Baker for BlockStates and custom models.
#[pyclass(name = "ModelBaker")]
#[derive(Default)]
pub struct PyModelBaker {
    pub(crate) inner: ModelBaker,
}

#[pymethods]
impl PyModelBaker {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: ModelBaker::new(),
        }
    }

    /// Clears internal model bake cache.
    pub fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }

    /// Prebakes ALL blockstates discovered across the entire resource pack stack in parallel.
    ///
    /// Automatically releases Python GIL during multi-threaded baking.
    pub fn bake_all(
        &self,
        py: Python<'_>,
        stack: &PyResourcePackStack,
    ) -> PyResult<PyBakedModelDatabase> {
        let db = py
            .allow_threads(|| libmtk::prebake_all_models(&stack.inner))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyBakedModelDatabase { inner: db })
    }

    /// Bakes a single blockstate string into a `PyMeshData` and texture list.
    ///
    /// # Arguments
    /// - `stack`: Resource pack stack to resolve blockstates and JSON models.
    /// - `state_str`: E.g. `"minecraft:oak_stairs[facing=east,half=bottom,shape=straight]"`
    /// - `clip_hidden`: Whether to clip faces occluded by interior element geometry.
    #[pyo3(signature = (stack, state_str, clip_hidden=true))]
    pub fn bake_blockstate(
        &mut self,
        stack: &PyResourcePackStack,
        state_str: &str,
        clip_hidden: bool,
    ) -> PyResult<(PyMeshData, Vec<String>)> {
        let bs = BlockState::parse(state_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let bs_def = {
            let bs_path = format!("assets/{}/blockstates/{}.json", bs.namespace, bs.name);
            stack.inner.open_asset_raw(&bs_path).and_then(|bytes| {
                serde_json::from_slice::<BlockStateDefinition>(&bytes).ok()
            })
        };

        let baked = self
            .inner
            .bake_blockstate(state_str, bs_def.as_ref(), |model_id| {
                let (ns, raw_path) = if let Some((ns, n)) = model_id.split_once(':') {
                    (ns, n)
                } else {
                    ("minecraft", model_id)
                };
                let candidates = [
                    format!("assets/{}/models/{}.json", ns, raw_path),
                    format!("assets/{}/models/block/{}.json", ns, raw_path),
                    format!("assets/{}/models/item/{}.json", ns, raw_path),
                ];
                for path in candidates {
                    if let Some(bytes) = stack.inner.open_asset_raw(&path) {
                        if let Ok(model) = serde_json::from_slice::<BlockModelJson>(&bytes) {
                            return Some(model);
                        }
                    }
                }
                None
            })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let (mesh, textures) = baked.to_mesh_with_textures(clip_hidden);
        Ok((PyMeshData { inner: mesh }, textures))
    }
}
