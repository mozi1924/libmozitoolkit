//! # Blender Direct Pointer Ingestion Interface
//!
//! Provides dedicated zero-copy raw memory pointer injection specifically tailored
//! for Blender DNA CustomData memory layouts.

use pyo3::prelude::*;

use super::PyMeshData;

/// Dedicated Blender hardware/pointer memory injection adapter.
#[pyclass(name = "BlenderDirectMesh")]
#[derive(Debug)]
pub struct PyBlenderDirectMesh {
    pub(crate) mesh: Py<PyMeshData>,
}

#[pymethods]
impl PyBlenderDirectMesh {
    /// Copies vertex positions directly into the destination host pointer (`float32 * 3` per vertex).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_positions(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_positions_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies vertex normals directly into the destination host pointer (`float32 * 3` per vertex).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_normals(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_normals_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies triangle indices directly into the destination host pointer (`uint32` per index).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_indices(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_indices_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies quad polygon indices directly into the destination host pointer (`uint32` per index).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_quad_indices(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_quad_indices_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies loop UVs directly into the destination host pointer (`float32 * 2` per loop).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_loop_uvs(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_loop_uvs_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies polygon loop start offsets directly into the destination host pointer (`int32` per polygon).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_loop_starts(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_loop_starts_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies polygon loop totals directly into the destination host pointer (`int32` per polygon).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_loop_totals(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_loop_totals_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies face material slots directly into the destination host pointer (`int32` per face).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_face_materials(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_face_materials_to_ptr(&borrowed, dst_ptr, max_bytes)
    }

    /// Copies vertex RGBA colors directly into the destination host pointer (`float32 * 4` per vertex).
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn copy_colors(
        &self,
        py: Python<'_>,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        let borrowed = self.mesh.borrow(py);
        crate::blender::direct_copy_colors_to_ptr(&borrowed, dst_ptr, max_bytes)
    }
}
