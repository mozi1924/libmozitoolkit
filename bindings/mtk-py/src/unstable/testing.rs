//! Test-only package fixtures exposed under `unstable.testing`.

use pyo3::prelude::*;

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
    let mut writer = mtk_package::MtkPackageFileWriter::create(
        p,
        mtk_package::PackageProfile::AssetCache,
        fp_bytes,
    )
    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    let manifest = libmtk::CacheManifest {
        format_version: libmtk::ASSET_CACHE_FORMAT_VERSION,
        fingerprint: fingerprint.to_string(),
        pack_count: 1,
        atlas_chunks: 1,
        standalone_textures: 0,
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

/// Registers the `testing` module on `parent_module`.
pub fn register_submodule(py: Python<'_>, parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(py, "testing")?;
    sub.add_function(wrap_pyfunction!(create_test_cache_package, &sub)?)?;
    parent_module.add_submodule(&sub)?;
    Ok(())
}
