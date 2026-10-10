//! # `mtk-py` Minecraft Resource Pack Binding
//!
//! Exposes layered resource pack loading and asset discovery to Python.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::path::Path;

#[cfg(feature = "zip")]
use mtk_resource::ZipPack;
use mtk_resource::{DirectoryPack, ResourceLocation, ResourcePackStack};

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
        self.inner.append_pack(Box::new(pack));
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
            self.inner.append_pack(Box::new(pack));
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
        format!("<ResourcePackStack packs={}>", self.inner.len())
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
    pub package_path: String,
    #[pyo3(get)]
    pub cache_dir: String,
}

#[pymethods]
impl PyPrecompileResult {
    fn __repr__(&self) -> String {
        format!(
            "<PrecompileResult packs={} atlas_chunks={} standalone={} models={} package='{}'>",
            self.pack_count,
            self.atlas_chunks,
            self.standalone_textures,
            self.baked_models,
            self.package_path
        )
    }
}

/// Reader and inspector for precompiled `.mtkcache` packages.
#[pyclass(name = "AssetCache")]
pub struct PyAssetCache {
    inner: libmtk::AssetCacheReader,
    package_path: String,
}

#[pymethods]
impl PyAssetCache {
    /// Opens an asset cache `.mtkcache` file using memory mapping.
    #[staticmethod]
    pub fn open(path: &str) -> PyResult<Self> {
        let reader = libmtk::AssetCacheReader::open(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(Self {
            inner: reader,
            package_path: path.to_string(),
        })
    }

    /// Validates whether a file at `path` matches the specified fingerprint using only the 64-byte header.
    #[staticmethod]
    pub fn is_valid(path: &str, fingerprint: &str) -> bool {
        libmtk::AssetCacheReader::is_valid_cache_file(path, fingerprint)
    }

    /// Absolute path to the opened package file.
    #[getter]
    pub fn package_path(&self) -> &str {
        &self.package_path
    }

    /// Full content fingerprint of the package.
    #[getter]
    pub fn fingerprint(&self) -> String {
        if let Ok(m) = self.inner.manifest() {
            m.fingerprint
        } else {
            let b = self.inner.fingerprint();
            format!("{:02x?}", b)
        }
    }

    /// Reads cache manifest metadata as a dictionary.
    pub fn get_manifest<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let m = self
            .inner
            .manifest()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let dict = PyDict::new(py);
        dict.set_item("format_version", m.format_version)?;
        dict.set_item("fingerprint", m.fingerprint)?;
        dict.set_item("pack_count", m.pack_count)?;
        dict.set_item("atlas_chunks", m.atlas_chunks)?;
        dict.set_item("standalone_textures", m.standalone_textures)?;
        dict.set_item("baked_models", m.baked_models)?;
        dict.set_item("created_at_epoch_secs", m.created_at_epoch_secs)?;
        Ok(dict)
    }

    /// Loads baked model database directly from package.
    pub fn load_models(&self) -> PyResult<crate::model::PyBakedModelDatabase> {
        let db = self
            .inner
            .load_models()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(crate::model::PyBakedModelDatabase {
            inner: std::sync::Arc::new(db),
        })
    }

    /// Loads baked atlas address map directly from package.
    pub fn load_atlas(&self) -> PyResult<crate::texture::PyBakedAtlas> {
        let map = self
            .inner
            .load_atlas_mapping()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(crate::texture::PyBakedAtlas {
            inner: mtk_texture::BakedAtlas {
                chunks: Vec::new(),
                address_map: map,
            },
        })
    }

    /// Loads biome resolver directly from package.
    pub fn load_biome_resolver(&self) -> PyResult<crate::material::PyBiomeResolver> {
        let resolver = self
            .inner
            .load_biome_resolver()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(crate::material::PyBiomeResolver { inner: resolver })
    }

    /// Extracts all atlas textures (`atlas/textures/*`) into the target directory.
    pub fn extract_atlas_textures(&self, output_dir: &str) -> PyResult<Vec<String>> {
        let paths = self
            .inner
            .extract_all_atlas_textures(output_dir)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(paths
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    }

    /// Extracts all standalone textures (`standalone/*`) into the target directory.
    pub fn extract_standalone_textures(&self, output_dir: &str) -> PyResult<Vec<String>> {
        let paths = self
            .inner
            .extract_all_standalone_textures(output_dir)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(paths
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    }

    /// Extracts all colormap textures into the target directory.
    pub fn extract_colormaps(&self, output_dir: &str) -> PyResult<Vec<String>> {
        let paths = self
            .inner
            .extract_all_colormaps(output_dir)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(paths
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    }

    /// Extracts a single texture chunk to the specified destination path.
    pub fn extract_atlas_texture(&self, filename: &str, output_path: &str) -> PyResult<()> {
        self.inner
            .extract_atlas_texture(filename, output_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Extracts a single standalone texture to the specified destination path.
    pub fn extract_standalone_texture(&self, rel_path: &str, output_path: &str) -> PyResult<()> {
        self.inner
            .extract_standalone_texture(rel_path, output_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Loads standalone mapping table JSON string directly from package.
    pub fn load_standalone_mapping(&self) -> PyResult<String> {
        self.inner
            .load_standalone_mapping_json()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Loads raw atlas mapping JSON string directly from package.
    pub fn load_atlas_mapping(&self) -> PyResult<String> {
        self.inner
            .load_atlas_mapping_json()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Reads and decodes a texture chunk directly in memory into RGBA8 bytes: `(width, height, memoryview)`.
    /// `flip_v`: If True, flips rows vertically (V-axis inversion) natively in Rust.
    #[pyo3(signature = (chunk_id, flip_v=false))]
    pub fn read_texture_rgba<'py>(
        &self,
        py: Python<'py>,
        chunk_id: &str,
        flip_v: bool,
    ) -> PyResult<(u32, u32, Bound<'py, pyo3::types::PyMemoryView>)> {
        let buf = self
            .inner
            .read_texture_rgba_with_orientation(chunk_id, flip_v)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let width = buf.width;
        let height = buf.height;
        let py_bytes = pyo3::types::PyBytes::new(py, &buf.pixels);
        let memview = pyo3::types::PyMemoryView::from(&py_bytes)?;
        Ok((width, height, memview))
    }

    /// Reads raw decompressed chunk bytes directly from package.
    pub fn read_chunk_bytes<'py>(
        &self,
        py: Python<'py>,
        chunk_id: &str,
    ) -> PyResult<Bound<'py, pyo3::types::PyBytes>> {
        let bytes = self
            .inner
            .read_chunk_bytes(chunk_id)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(pyo3::types::PyBytes::new(py, &bytes))
    }

    /// Extracts a single colormap texture to the specified destination path.
    pub fn extract_colormap(&self, name: &str, output_path: &str) -> PyResult<()> {
        self.inner
            .extract_colormap(name, output_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }
}

/// Checks if a `.mtkcache` file at `path` is valid for `fingerprint` using 64-byte header.
#[pyfunction]
pub fn is_valid_cache_package(path: &str, fingerprint: &str) -> bool {
    libmtk::AssetCacheReader::is_valid_cache_file(path, fingerprint)
}

/// Reads the manifest of a cache package or searches for one in a directory.
#[pyfunction]
pub fn get_cache_manifest<'py>(
    py: Python<'py>,
    path: &str,
) -> PyResult<Option<Bound<'py, PyDict>>> {
    let p = std::path::Path::new(path);
    if let Some(m) = libmtk::CacheManifest::read_from_dir(p) {
        let dict = PyDict::new(py);
        dict.set_item("format_version", m.format_version)?;
        dict.set_item("fingerprint", m.fingerprint)?;
        dict.set_item("pack_count", m.pack_count)?;
        dict.set_item("atlas_chunks", m.atlas_chunks)?;
        dict.set_item("standalone_textures", m.standalone_textures)?;
        dict.set_item("baked_models", m.baked_models)?;
        dict.set_item("created_at_epoch_secs", m.created_at_epoch_secs)?;
        Ok(Some(dict))
    } else {
        Ok(None)
    }
}

/// Executes unified end-to-end asset precompilation directly to persistent cache folder.
///
/// Releases Python GIL during multi-threaded baking, re-acquiring for progress callbacks.
#[pyfunction]
#[pyo3(signature = (stack, cache_dir, atlas_category="blocks", max_atlas_width=4096, max_atlas_height=4096, compile_atlas=true, compile_standalone=true, compile_models=true, num_threads=None, callback=None))]
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
    num_threads: Option<usize>,
    callback: Option<PyObject>,
) -> PyResult<PyPrecompileResult> {
    let cfg = libmtk::PrecompileConfig {
        max_atlas_width,
        max_atlas_height,
        atlas_category: atlas_category.to_string(),
        compile_atlas,
        compile_standalone,
        compile_models,
        num_threads,
    };

    let cb_opt = callback.as_ref();
    let on_progress = cb_opt.map(|cb| {
        move |prog: mtk_core::progress::ProgressReport| {
            Python::with_gil(|py| {
                let dict = PyDict::new(py);
                let _ = dict.set_item("stage", prog.stage);
                let _ = dict.set_item("current", prog.current);
                let _ = dict.set_item("total", prog.total);
                let _ = dict.set_item("message", &prog.message);
                let _ = dict.set_item("percent", prog.percent());
                let _ = cb.call1(py, (dict,));
            });
        }
    });
    let progress_ref: Option<mtk_core::progress::ProgressCallback<'_>> = on_progress
        .as_ref()
        .map(|f| f as &(dyn Fn(mtk_core::progress::ProgressReport) + Send + Sync));

    let res = py
        .allow_threads(|| {
            libmtk::precompile_all_assets_with_progress(&stack.inner, cache_dir, &cfg, progress_ref)
        })
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyPrecompileResult {
        success: res.success,
        pack_count: res.pack_count,
        atlas_chunks: res.atlas_chunks,
        standalone_textures: res.standalone_textures,
        baked_models: res.baked_models,
        fingerprint: res.fingerprint,
        package_path: res.package_path,
        cache_dir: res.cache_dir,
    })
}

/// Helper function for unit testing: creates a minimal valid `.mtkcache` package file.
#[pyfunction]
#[pyo3(signature = (path, fingerprint, created_at=0))]
pub fn create_test_cache_package(
    path: &str,
    fingerprint: &str,
    created_at: u64,
) -> PyResult<String> {
    let p = std::path::Path::new(path);
    let fp_bytes = libmtk::fingerprint_str_to_bytes16(fingerprint);
    let mut writer =
        mtk_package::MtkPackageWriter::create(p, mtk_package::PackageProfile::AssetCache, fp_bytes)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    let manifest = libmtk::CacheManifest {
        format_version: libmtk::ASSET_CACHE_FORMAT_VERSION,
        fingerprint: fingerprint.to_string(),
        pack_count: 1,
        atlas_chunks: 1,
        standalone_textures: 10,
        baked_models: 50,
        created_at_epoch_secs: created_at,
    };
    let json_str = serde_json::to_string(&manifest)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    writer
        .add_str_chunk(
            *b"META",
            "manifest",
            &json_str,
            &mtk_package::ChunkWriteOptions::zstd_fast(),
        )
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    Ok(p.to_string_lossy().to_string())
}
