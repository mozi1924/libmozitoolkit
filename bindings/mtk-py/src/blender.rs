//! Backward-compatible Blender direct pointer module delegating to `mtk-blender-dna`.

use pyo3::prelude::*;

use crate::mesh::PyMeshData;
use crate::point_cloud::PyVoxelPointCloud;
use crate::unstable::blender_dna::map_dna_err;

pub use crate::unstable::blender_dna::copy_buffer_to_ptr;

#[inline]
pub fn validate_pointer_and_bounds(
    dst_ptr: usize,
    required_bytes: usize,
    max_bytes: Option<usize>,
) -> PyResult<()> {
    mtk_blender_dna::validate_pointer_and_bounds(dst_ptr, required_bytes, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_positions_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_positions_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_normals_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_normals_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_indices_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_indices_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_quad_indices_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_quad_indices_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_loop_uvs_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_loop_uvs_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_face_materials_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_face_materials_to_ptr(&mesh.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_loop_starts_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_loop_starts_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_loop_totals_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_loop_totals_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_colors_to_ptr(
    mesh: &PyMeshData,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_colors_to_ptr(&mesh.inner, dst_ptr, max_bytes).map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_positions_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_positions_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_block_x_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_block_x_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_block_y_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_block_y_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_block_z_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_block_z_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_state_indices_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<(Vec<String>, usize)> {
    mtk_blender_dna::copy_point_cloud_state_indices_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_biome_indices_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<(Vec<String>, usize)> {
    mtk_blender_dna::copy_point_cloud_biome_indices_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_light_levels_to_ptr(
    cloud: &PyVoxelPointCloud,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    mtk_blender_dna::copy_point_cloud_light_levels_to_ptr(&cloud.inner, dst_ptr, max_bytes)
        .map_err(map_dna_err)
}

pub fn direct_copy_point_cloud_to_ptrs(
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

/// Registers the legacy `blender` submodule on `parent_module` for backward compatibility.
pub fn register_submodule(py: Python<'_>, parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let sub = PyModule::new(py, "blender")?;
    sub.add_function(wrap_pyfunction!(copy_buffer_to_ptr, &sub)?)?;
    parent_module.add_submodule(&sub)?;
    Ok(())
}
