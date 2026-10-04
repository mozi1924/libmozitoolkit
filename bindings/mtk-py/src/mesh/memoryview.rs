//! Zero-copy memoryview and buffer protocol exports for `MeshData`.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyMemoryView};

use super::PyMeshData;

/// Read-only memoryview of vertex positions as raw bytes (`float32 * 3` per vertex).
pub fn positions_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    if mesh.inner.positions.is_empty() {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            mesh.inner.positions.as_ptr() as *const u8,
            mesh.inner.positions.len() * std::mem::size_of::<[f32; 3]>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of vertex normals as raw bytes (`float32 * 3` per vertex).
pub fn normals_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    if mesh.inner.normals.is_empty() {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            mesh.inner.normals.as_ptr() as *const u8,
            mesh.inner.normals.len() * std::mem::size_of::<[f32; 3]>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of vertex UV coordinates as raw bytes (`float32 * 2` per vertex).
pub fn uvs_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    if mesh.inner.uvs.is_empty() {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            mesh.inner.uvs.as_ptr() as *const u8,
            mesh.inner.uvs.len() * std::mem::size_of::<[f32; 2]>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of secondary normalized [0, 1] vertex UVs (if generated).
pub fn secondary_uvs_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
    if let Some(ref sec_uvs) = mesh.inner.secondary_uvs {
        if sec_uvs.is_empty() {
            return Ok(None);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                sec_uvs.as_ptr() as *const u8,
                sec_uvs.len() * std::mem::size_of::<[f32; 2]>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        Ok(Some(PyMemoryView::from(&bytes)?))
    } else {
        Ok(None)
    }
}

/// Flattened secondary vertex UVs `[u0, v0, u1, v1, ...]`.
pub fn get_flat_secondary_uvs<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Option<Bound<'py, PyList>>> {
    if let Some(ref sec_uvs) = mesh.inner.secondary_uvs {
        let mut flat = Vec::with_capacity(sec_uvs.len() * 2);
        for uv in sec_uvs {
            flat.push(uv[0]);
            flat.push(uv[1]);
        }
        Ok(Some(PyList::new(py, &flat)?))
    } else {
        Ok(None)
    }
}

/// Read-only memoryview of triangle indices as raw bytes (`uint32` per index).
pub fn indices_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    if mesh.inner.indices.is_empty() {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            mesh.inner.indices.as_ptr() as *const u8,
            mesh.inner.indices.len() * std::mem::size_of::<u32>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of quad indices as raw bytes (`uint32` per index) if quads are recorded.
pub fn quad_indices_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
    if let Some(ref quads) = mesh.inner.quad_indices {
        if quads.is_empty() {
            return Ok(None);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                quads.as_ptr() as *const u8,
                quads.len() * std::mem::size_of::<u32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        Ok(Some(PyMemoryView::from(&bytes)?))
    } else {
        Ok(None)
    }
}

/// Read-only memoryview of expanded per-loop UV coordinates (`float32 * 2` per loop)
/// aligned directly with Blender's polygon loops.
pub fn loop_uvs_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);

    let total_loops = if is_quad {
        if let Some(ref quads) = mesh.inner.quad_indices {
            quads.len()
        } else {
            (mesh.inner.indices.len() / 6) * 4
        }
    } else {
        mesh.inner.indices.len()
    };

    if total_loops == 0 {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }

    // When mesh.inner.uvs already stores per-corner (loop) UVs (e.g. from SectionMesher,
    // append_quad, or where spatial welding preserved corner UVs), its length matches total_loops.
    // Return a zero-copy memoryview directly over uvs_flat() without re-indexing!
    if mesh.inner.uvs.len() == total_loops {
        let uvs_flat = mesh.inner.uvs_flat();
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                uvs_flat.as_ptr() as *const u8,
                uvs_flat.len() * std::mem::size_of::<f32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        return PyMemoryView::from(&bytes);
    }

    let loop_uvs: Vec<f32> = py.allow_threads(|| {
        if is_quad {
            let quads = mesh.inner.reconstruct_quad_indices();
            let mut uvs = Vec::with_capacity(quads.len() * 2);
            for &v_idx in &quads {
                if let Some(uv) = mesh.inner.uvs.get(v_idx as usize) {
                    uvs.push(uv[0]);
                    uvs.push(uv[1]);
                } else {
                    uvs.push(0.0);
                    uvs.push(0.0);
                }
            }
            uvs
        } else if !mesh.inner.indices.is_empty() {
            let mut uvs = Vec::with_capacity(mesh.inner.indices.len() * 2);
            for &v_idx in &mesh.inner.indices {
                if let Some(uv) = mesh.inner.uvs.get(v_idx as usize) {
                    uvs.push(uv[0]);
                    uvs.push(uv[1]);
                } else {
                    uvs.push(0.0);
                    uvs.push(0.0);
                }
            }
            uvs
        } else {
            let mut uvs = Vec::with_capacity(mesh.inner.uvs.len() * 2);
            for uv in &mesh.inner.uvs {
                uvs.push(uv[0]);
                uvs.push(uv[1]);
            }
            uvs
        }
    });

    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            loop_uvs.as_ptr() as *const u8,
            loop_uvs.len() * std::mem::size_of::<f32>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of polygon loop start offsets (`int32` per polygon) for Blender direct topology injection.
pub fn loop_starts_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);
    let count = if is_quad {
        if let Some(ref quads) = mesh.inner.quad_indices {
            quads.len() / 4
        } else {
            mesh.inner.indices.len() / 6
        }
    } else {
        mesh.inner.indices.len() / 3
    };
    if count == 0 {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let step = if is_quad { 4i32 } else { 3i32 };
    let mut starts: Vec<i32> = Vec::with_capacity(count);
    for i in 0..count {
        starts.push(i as i32 * step);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            starts.as_ptr() as *const u8,
            starts.len() * std::mem::size_of::<i32>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of polygon loop totals (`int32` per polygon: 4 for quads, 3 for triangles).
pub fn loop_totals_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);
    let count = if is_quad {
        if let Some(ref quads) = mesh.inner.quad_indices {
            quads.len() / 4
        } else {
            mesh.inner.indices.len() / 6
        }
    } else {
        mesh.inner.indices.len() / 3
    };
    if count == 0 {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let val = if is_quad { 4i32 } else { 3i32 };
    let totals = vec![val; count];
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            totals.as_ptr() as *const u8,
            totals.len() * std::mem::size_of::<i32>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of face material slots (`uint16` per face).
pub fn face_materials_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    if mesh.inner.face_materials.is_empty() {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            mesh.inner.face_materials.as_ptr() as *const u8,
            mesh.inner.face_materials.len() * std::mem::size_of::<u16>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of face tint indices (`int8` per face).
pub fn face_tint_indices_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
    if mesh.inner.face_tint_indices.is_empty() {
        let bytes = PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let byte_slice = unsafe {
        std::slice::from_raw_parts(
            mesh.inner.face_tint_indices.as_ptr() as *const u8,
            mesh.inner.face_tint_indices.len() * std::mem::size_of::<i8>(),
        )
    };
    let bytes = PyBytes::new(py, byte_slice);
    PyMemoryView::from(&bytes)
}

/// Read-only memoryview of vertex RGBA colors (`float32 * 4` per vertex) if present.
pub fn colors_memoryview<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
    if let Some(ref cols) = mesh.inner.colors {
        if cols.is_empty() {
            return Ok(None);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                cols.as_ptr() as *const u8,
                cols.len() * std::mem::size_of::<[f32; 4]>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        Ok(Some(PyMemoryView::from(&bytes)?))
    } else {
        Ok(None)
    }
}
