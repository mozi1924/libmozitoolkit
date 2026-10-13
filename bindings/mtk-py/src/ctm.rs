//! OptiFine / Continuity Connected Textures (CTM) Python bindings.

use glam::IVec3;
use mtk_core::direction::Direction;
use mtk_resource::{CtmSolver, ResourceLocation};
use pyo3::prelude::*;
use std::sync::Arc;

/// Python wrapper for `CtmSolver` connected texture pattern solver.
#[pyclass(name = "CtmSolver")]
#[derive(Clone, Default)]
pub struct PyCtmSolver {
    pub(crate) inner: Arc<CtmSolver>,
}

#[pymethods]
impl PyCtmSolver {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CtmSolver::default()),
        }
    }

    /// Number of active CTM rules loaded in this solver.
    #[getter]
    pub fn rule_count(&self) -> usize {
        self.inner.rules.len()
    }

    /// Resolves the CTM texture override for a block face.
    #[pyo3(signature = (state, face, x, y, z, base_tile=None, biome=None))]
    pub fn resolve_face(
        &self,
        state: &str,
        face: &str,
        x: i32,
        y: i32,
        z: i32,
        base_tile: Option<&str>,
        biome: Option<&str>,
    ) -> Option<String> {
        let dir = Direction::parse_loose(face).unwrap_or(Direction::North);
        let pos = IVec3::new(x, y, z);
        let base_loc = base_tile.and_then(|bt| ResourceLocation::parse(bt).ok());
        let dummy = |_: IVec3| None::<&str>;
        self.inner
            .resolve_face(state, dir, pos, base_loc.as_ref(), biome, dummy)
            .map(|loc| loc.as_string())
    }

    fn __repr__(&self) -> String {
        format!("<CtmSolver rules={}>", self.inner.rules.len())
    }
}
