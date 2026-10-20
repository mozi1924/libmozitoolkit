//! Python bindings for `mtk-blender-dna` direct memory pointer adapters.
//!
//! Exposes unstable, hardware-accelerated direct memory and DNA pointer manipulation
//! utilities for Blender integration.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::mesh::PyMeshData;
use crate::point_cloud::PyVoxelPointCloud;

#[inline]
pub(crate) fn map_dna_err(err: mtk_blender_dna::DnaError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

/// Directly copies vertex positions into a contiguous destination pointer as raw float32 values (`len * 3`).
#[pyfunction]
#[pyo3(signature = (mesh, dst_ptr, max_bytes=None))]
pub fn copy_positions_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_positions_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

/// Directly copies vertex normals into a contiguous destination pointer as raw float32 values (`len * 3`).
#[pyfunction]
#[pyo3(signature = (mesh, dst_ptr, max_bytes=None))]
pub fn copy_normals_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_normals_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

/// Directly copies triangle indices into a contiguous destination pointer as raw uint32 values.
#[pyfunction]
#[pyo3(signature = (mesh, dst_ptr, max_bytes=None))]
pub fn copy_indices_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_indices_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

/// Directly copies reconstructed quad indices into a contiguous destination pointer as raw uint32 values.
#[pyfunction]
#[pyo3(signature = (mesh, dst_ptr, max_bytes=None))]
pub fn copy_quad_indices_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_quad_indices_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

/// Directly copies expanded per-loop UVs into a contiguous destination pointer as raw float32 pairs.
#[pyfunction]
#[pyo3(signature = (mesh, dst_ptr, max_bytes=None))]
pub fn copy_loop_uvs_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_loop_uvs_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

/// Directly copies vertex RGBA colors into the destination pointer as raw float32 values (`len * 4`).
#[pyfunction]
#[pyo3(signature = (mesh, dst_ptr, max_bytes=None))]
pub fn copy_colors_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_colors_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

/// Directly copies voxel point cloud 3D positions into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_positions_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_positions_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Directly copies discrete integer block X coordinates into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_block_x_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_block_x_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Directly copies discrete integer block Y coordinates into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_block_y_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_block_y_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Directly copies discrete integer block Z coordinates into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_block_z_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_block_z_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Directly converts block states into a palette and copies int32 palette indices into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_state_indices_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<(Vec<String>, usize)> {
    mtk_blender_dna::copy_point_cloud_state_indices_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Directly converts biomes into a palette and copies int32 palette indices into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_biome_indices_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<(Vec<String>, usize)> {
    mtk_blender_dna::copy_point_cloud_biome_indices_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Directly copies light levels into a destination pointer.
#[pyfunction]
#[pyo3(signature = (cloud, dst_ptr, max_bytes=None))]
pub fn copy_point_cloud_light_levels_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_light_levels_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

/// Batch direct copy of all available voxel point cloud attributes in a single call.
#[pyfunction]
#[pyo3(signature = (cloud, pos_ptr=None, block_x_ptr=None, block_y_ptr=None, block_z_ptr=None, state_idx_ptr=None, biome_idx_ptr=None, light_ptr=None))]
pub fn copy_point_cloud_to_ptrs(
    cloud: &PyVoxelPointCloud,
    pos_ptr: Option<usize>,
    block_x_ptr: Option<usize>,
    block_y_ptr: Option<usize>,
    block_z_ptr: Option<usize>,
    state_idx_ptr: Option<usize>,
    biome_idx_ptr: Option<usize>,
    light_ptr: Option<usize>,
) -> PyResult<(Option<Vec<String>>, Option<Vec<String>>)> {
    mtk_blender_dna::copy_point_cloud_to_ptrs(
        &cloud.inner,
        pos_ptr,
        block_x_ptr,
        block_y_ptr,
        block_z_ptr,
        state_idx_ptr,
        biome_idx_ptr,
        light_ptr,
    )
    .map_err(map_dna_err)
}

/// Registers the `blender_dna` module on `parent_module`.
pub fn register_submodule(py: Python<'_>, parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(py, "blender_dna")?;
    sub.add_function(wrap_pyfunction!(copy_positions_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_normals_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_indices_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_quad_indices_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_loop_uvs_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_colors_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_point_cloud_positions_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_point_cloud_block_x_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_point_cloud_block_y_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(copy_point_cloud_block_z_to_ptr, &sub)?)?;
    sub.add_function(wrap_pyfunction!(
        copy_point_cloud_state_indices_to_ptr,
        &sub
    )?)?;
    sub.add_function(wrap_pyfunction!(
        copy_point_cloud_biome_indices_to_ptr,
        &sub
    )?)?;
    sub.add_function(wrap_pyfunction!(
        copy_point_cloud_light_levels_to_ptr,
        &sub
    )?)?;
    sub.add_function(wrap_pyfunction!(copy_point_cloud_to_ptrs, &sub)?)?;

    parent_module.add_submodule(&sub)?;
    Ok(())
}
