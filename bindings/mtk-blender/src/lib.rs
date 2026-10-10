//! # `libmtk_blender` (Dedicated Blender Direct-Memory Accelerator)
//!
//! Provides high-performance, bounds-guarded direct memory transfers between
//! MoziToolKit contiguous buffer objects and Blender CustomData memory pointers.

pub mod mesh;

use pyo3::prelude::*;

/// Python module initialization for `libmtk_blender`.
#[pymodule]
fn libmtk_blender(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(mesh::copy_buffer_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_positions_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_normals_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_indices_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_quad_indices_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_loop_uvs_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_loop_starts_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_loop_totals_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_face_materials_to_ptr, m)?)?;
    m.add_function(wrap_pyfunction!(mesh::copy_colors_to_ptr, m)?)?;

    Ok(())
}
