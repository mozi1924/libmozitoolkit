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
            let pack = ZipPack::from_file(path, p)
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

    /// Computes a deterministic fingerprint string for the active resource pack stack configuration.
    pub fn compute_stack_fingerprint(&self) -> String {
        self.inner.compute_stack_fingerprint()
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

/// Result of full-scale asset precompilation.
#[pyclass(name = "PrecompileResult")]
#[derive(Debug, Clone)]
pub struct PyPrecompileResult {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub pack_count: usize,
    #[pyo3(get)]
    pub atlas_chunks: usize,
    #[pyo3(get)]
    pub standalone_textures: usize,
    #[pyo3(get)]
    pub baked_models: usize,
    #[pyo3(get)]
    pub fingerprint: String,
    #[pyo3(get)]
    pub cache_dir: String,
}

#[pymethods]
impl PyPrecompileResult {
    fn __repr__(&self) -> String {
        format!(
            "<PrecompileResult packs={} atlas_chunks={} standalone={} models={} cache_dir='{}'>",
            self.pack_count, self.atlas_chunks, self.standalone_textures, self.baked_models, self.cache_dir
        )
    }
}

/// Executes unified end-to-end asset precompilation directly to persistent cache folder.
///
/// Releases Python GIL during multi-threaded baking.
#[pyfunction]
#[pyo3(signature = (stack, cache_dir, atlas_category="blocks", max_atlas_width=4096, max_atlas_height=4096, compile_atlas=true, compile_standalone=true, compile_models=true))]
pub fn precompile_all_assets<'py>(
    py: Python<'py>,
    stack: &PyResourcePackStack,
    cache_dir: &str,
    atlas_category: &str,
    max_atlas_width: u32,
    max_atlas_height: u32,
    compile_atlas: bool,
    compile_standalone: bool,
    compile_models: bool,
) -> PyResult<PyPrecompileResult> {
    let cfg = libmtk::PrecompileConfig {
        max_atlas_width,
        max_atlas_height,
        atlas_category: atlas_category.to_string(),
        compile_atlas,
        compile_standalone,
        compile_models,
    };

    let res = py
        .allow_threads(|| libmtk::precompile_all_assets(&stack.inner, cache_dir, &cfg))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyPrecompileResult {
        success: res.success,
        pack_count: res.pack_count,
        atlas_chunks: res.atlas_chunks,
        standalone_textures: res.standalone_textures,
        baked_models: res.baked_models,
        fingerprint: res.fingerprint,
        cache_dir: res.cache_dir,
    })
}
