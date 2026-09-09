//! # `mtk-py` Minecraft Resource Pack Binding
//!
//! Exposes layered resource pack loading and asset discovery to Python.

use std::path::Path;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use mtk_resource::{DirectoryPack, ResourceLocation, ResourcePackStack};
#[cfg(feature = "zip")]
use mtk_resource::ZipPack;

/// Python wrapper for layered Minecraft resource packs (`ResourcePackStack`).
#[pyclass(name = "ResourcePackStack")]
#[derive(Default)]
pub struct PyResourcePackStack {
    pub(crate) inner: ResourcePackStack,
}

#[pymethods]
impl PyResourcePackStack {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: ResourcePackStack::new(),
        }
    }

    /// Adds a folder/directory resource pack to the stack.
    #[pyo3(signature = (path, name=None))]
    pub fn add_directory_pack(&mut self, path: &str, name: Option<&str>) -> PyResult<bool> {
        let p = Path::new(path);
        if !p.exists() || !p.is_dir() {
            return Err(pyo3::exceptions::PyFileNotFoundError::new_err(format!(
                "Directory does not exist: {}",
                path
            )));
        }
        let pack_name = name.unwrap_or(path);
        let pack = DirectoryPack::new(pack_name, p);
        self.inner.push_pack(Box::new(pack));
        Ok(true)
    }

    /// Adds a `.zip` or `.jar` archive resource pack to the stack.
    pub fn add_zip_pack(&mut self, path: &str) -> PyResult<bool> {
        #[cfg(feature = "zip")]
        {
            let p = Path::new(path);
            if !p.exists() || !p.is_file() {
                return Err(pyo3::exceptions::PyFileNotFoundError::new_err(format!(
                    "ZIP archive file does not exist: {}",
                    path
                )));
            }
            let pack = ZipPack::from_file(p)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            self.inner.push_pack(Box::new(pack));
            Ok(true)
        }
        #[cfg(not(feature = "zip"))]
        {
            let _ = path;
            Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "ZIP support was not compiled in this build of mtk-py",
            ))
        }
    }

    /// Returns the number of loaded resource packs in the stack.
    pub fn get_pack_count(&self) -> usize {
        self.inner.len()
    }

    /// Loads raw PNG bytes for a given resource location string (e.g. `"minecraft:block/stone"`).
    pub fn open_texture_bytes<'py>(
        &self,
        py: Python<'py>,
        location: &str,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        let loc = ResourceLocation::parse(location)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(self
            .inner
            .open_texture_raw(&loc)
            .map(|bytes| PyBytes::new(py, &bytes)))
    }

    fn __repr__(&self) -> String {
        format!(
            "<ResourcePackStack packs={}>",
            self.inner.len()
        )
    }
}
