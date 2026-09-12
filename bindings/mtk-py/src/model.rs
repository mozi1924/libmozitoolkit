//! # `mtk-py` Model Baker Binding
//!
//! Exposes universal headless BlockState and Minecraft model baking to Python.

use pyo3::prelude::*;
use mtk_model::{BlockModelJson, BlockState, BlockStateDefinition, ModelBaker};
use crate::mesh::PyMeshData;
use crate::resource::PyResourcePackStack;

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
