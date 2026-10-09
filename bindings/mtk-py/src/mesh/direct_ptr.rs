//! Direct pointer-based zero-copy memory transfers between `PyMeshData` and host memory buffers.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use super::PyMeshData;

/// Directly copies vertex positions into a contiguous destination pointer as raw float32 values (`len * 3`).
pub fn direct_copy_positions_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let count = mesh.inner.positions.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<[f32; 3]>();
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.inner.positions.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies vertex normals into a contiguous destination pointer as raw float32 values (`len * 3`).
pub fn direct_copy_normals_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let count = mesh.inner.normals.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<[f32; 3]>();
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.inner.normals.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies triangle indices into a contiguous destination pointer as raw uint32 values.
pub fn direct_copy_indices_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let count = mesh.inner.indices.len();
    if count == 0 {
        return Ok(0);
    }
    let byte_size = count * std::mem::size_of::<u32>();
    unsafe {
        std::ptr::copy_nonoverlapping(
            mesh.inner.indices.as_ptr() as *const u8,
            dst_ptr as *mut u8,
            byte_size,
        );
    }
    Ok(byte_size)
}

/// Directly copies reconstructed quad indices into a contiguous destination pointer as raw uint32 values.
pub fn direct_copy_quad_indices_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let quads = mesh.inner.reconstruct_quad_indices();
    if quads.is_empty() {
        return Ok(0);
    }
    let byte_size = quads.len() * std::mem::size_of::<u32>();
    unsafe {
        std::ptr::copy_nonoverlapping(quads.as_ptr() as *const u8, dst_ptr as *mut u8, byte_size);
    }
    Ok(byte_size)
}

/// Directly copies expanded per-loop UVs into a contiguous destination pointer as raw float32 pairs.
pub fn direct_copy_loop_uvs_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
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
        return Ok(0);
    }

    if mesh.inner.uvs.len() == total_loops {
        let uvs_flat = mesh.inner.uvs_flat();
        let byte_size = std::mem::size_of_val(uvs_flat);
        unsafe {
            std::ptr::copy_nonoverlapping(
                uvs_flat.as_ptr() as *const u8,
                dst_ptr as *mut u8,
                byte_size,
            );
        }
        return Ok(byte_size);
    }

    let dst = dst_ptr as *mut f32;
    let mut offset = 0usize;
    if is_quad {
        let quads = mesh.inner.reconstruct_quad_indices();
        for &v_idx in &quads {
            let uv = mesh
                .inner
                .uvs
                .get(v_idx as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
            unsafe {
                *dst.add(offset) = uv[0];
                *dst.add(offset + 1) = uv[1];
            }
            offset += 2;
        }
    } else {
        for &v_idx in &mesh.inner.indices {
            let uv = mesh
                .inner
                .uvs
                .get(v_idx as usize)
                .copied()
                .unwrap_or([0.0, 0.0]);
            unsafe {
                *dst.add(offset) = uv[0];
                *dst.add(offset + 1) = uv[1];
            }
            offset += 2;
        }
    }
    Ok(offset * std::mem::size_of::<f32>())
}

/// Directly copies face material slots converted to 32-bit integers into the destination pointer.
pub fn direct_copy_face_materials_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let count = mesh.inner.face_materials.len();
    if count == 0 {
        return Ok(0);
    }
    let dst = dst_ptr as *mut i32;
    for (i, &mat) in mesh.inner.face_materials.iter().enumerate() {
        unsafe {
            *dst.add(i) = mat as i32;
        }
    }
    Ok(count * std::mem::size_of::<i32>())
}

/// Directly copies polygon loop start offsets into the destination pointer as 32-bit integers.
pub fn direct_copy_loop_starts_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);
    let poly_count = if is_quad {
        mesh.inner
            .face_materials
            .len()
            .max(mesh.inner.indices.len() / 6)
    } else {
        mesh.inner.indices.len() / 3
    };
    if poly_count == 0 {
        return Ok(0);
    }
    let stride = if is_quad { 4i32 } else { 3i32 };
    let dst = dst_ptr as *mut i32;
    for i in 0..poly_count {
        unsafe {
            *dst.add(i) = (i as i32) * stride;
        }
    }
    Ok(poly_count * std::mem::size_of::<i32>())
}

/// Directly copies polygon loop totals into the destination pointer as 32-bit integers.
pub fn direct_copy_loop_totals_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    let is_quad = mesh.inner.quad_indices.is_some()
        || (!mesh.inner.indices.is_empty()
            && mesh.inner.indices.len() % 6 == 0
            && mesh.inner.face_materials.len() == mesh.inner.indices.len() / 6);
    let poly_count = if is_quad {
        mesh.inner
            .face_materials
            .len()
            .max(mesh.inner.indices.len() / 6)
    } else {
        mesh.inner.indices.len() / 3
    };
    if poly_count == 0 {
        return Ok(0);
    }
    let stride = if is_quad { 4i32 } else { 3i32 };
    let dst = dst_ptr as *mut i32;
    for i in 0..poly_count {
        unsafe {
            *dst.add(i) = stride;
        }
    }
    Ok(poly_count * std::mem::size_of::<i32>())
}

/// Directly copies vertex RGBA colors into the destination pointer as raw float32 values (`len * 4`).
pub fn direct_copy_colors_to_ptr(mesh: &PyMeshData, dst_ptr: usize) -> PyResult<usize> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err("Destination pointer cannot be null"));
    }
    if let Some(ref colors) = mesh.inner.colors {
        if colors.is_empty() {
            return Ok(0);
        }
        let byte_size = colors.len() * std::mem::size_of::<[f32; 4]>();
        unsafe {
            std::ptr::copy_nonoverlapping(
                colors.as_ptr() as *const u8,
                dst_ptr as *mut u8,
                byte_size,
            );
        }
        Ok(byte_size)
    } else {
        Ok(0)
    }
}
