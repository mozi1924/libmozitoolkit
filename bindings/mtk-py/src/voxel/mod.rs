//! Voxel storage, configuration, and world bindings.

use pyo3::prelude::*;

pub mod config;
pub mod storage;
pub mod world;

pub use config::*;
pub use storage::*;
pub use world::*;

/// Maps voxel-domain errors onto the Python exception types hosts can act on.
pub(crate) fn map_voxel_err(e: mtk_voxel::VoxelError) -> PyErr {
    match e {
        mtk_voxel::VoxelError::MissingVanillaBasePack(msg)
        | mtk_voxel::VoxelError::MissingAssets(msg) => pyo3::exceptions::PyValueError::new_err(msg),
        _ => pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
    }
}
