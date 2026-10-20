//! Unstable and experimental APIs for DCC host integration.
//!
//! Items under this module are non-standard and subject to breaking changes.
//! Host-specific pointer adapters are shipped as separate wheels and are not
//! part of this core module.

pub mod testing;

use pyo3::prelude::*;

/// Registers the `unstable` parent submodule on `parent_module`.
pub fn register_submodule(py: Python<'_>, parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let unstable = PyModule::new(py, "unstable")?;

    testing::register_submodule(py, &unstable)?;
    parent_module.add_submodule(&unstable)?;
    Ok(())
}
