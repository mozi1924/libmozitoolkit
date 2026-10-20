//! Unstable and experimental APIs for DCC host integration.
//!
//! Items under this module are non-standard, may depend on host-specific memory layouts
//! (such as Blender DNA), and are subject to breaking changes across host versions.

#[cfg(feature = "blender")]
pub mod blender_dna;
pub mod testing;

use pyo3::prelude::*;

/// Registers the `unstable` parent submodule on `parent_module`.
pub fn register_submodule(py: Python<'_>, parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let unstable = PyModule::new(py, "unstable")?;

    #[cfg(feature = "blender")]
    {
        blender_dna::register_submodule(py, &unstable)?;
    }

    testing::register_submodule(py, &unstable)?;
    parent_module.add_submodule(&unstable)?;
    Ok(())
}
