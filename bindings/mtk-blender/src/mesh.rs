//! Hardware-accelerated direct memory transfer routines into Blender CustomData memory buffers.

use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[inline]
fn validate_pointer_and_bounds(
    dst_ptr: usize,
    required_bytes: usize,
    max_bytes: Option<usize>,
) -> PyResult<()> {
    if dst_ptr == 0 {
        return Err(PyValueError::new_err(
            "Destination pointer cannot be null (0)",
        ));
    }
    if let Some(limit) = max_bytes {
        if required_bytes > limit {
            return Err(PyValueError::new_err(format!(
                "Direct copy buffer overflow protection: required {} bytes, but destination max_bytes is {}",
                required_bytes, limit
            )));
        }
    }
    Ok(())
}

/// Copies raw contiguous buffer bytes directly into a destination memory pointer with bounds checking.
#[pyfunction]
#[pyo3(signature = (src_buffer, dst_ptr, max_bytes=None))]
pub fn copy_buffer_to_ptr(
    src_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let py_buf: PyBuffer<u8> = PyBuffer::get(src_buffer)?;
    let len = py_buf.len_bytes();
    if len == 0 {
        return Ok(0);
    }
    validate_pointer_and_bounds(dst_ptr, len, max_bytes)?;
    unsafe {
        std::ptr::copy_nonoverlapping(py_buf.buf_ptr() as *const u8, dst_ptr as *mut u8, len);
    }
    Ok(len)
}

/// Copies vertex positions (`float32 * 3` per vertex) directly into destination pointer.
#[pyfunction]
#[pyo3(signature = (positions_buffer, dst_ptr, max_bytes=None))]
pub fn copy_positions_to_ptr(
    positions_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    copy_buffer_to_ptr(positions_buffer, dst_ptr, max_bytes)
}

/// Copies vertex normals (`float32 * 3` per vertex) directly into destination pointer.
#[pyfunction]
#[pyo3(signature = (normals_buffer, dst_ptr, max_bytes=None))]
pub fn copy_normals_to_ptr(
    normals_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    copy_buffer_to_ptr(normals_buffer, dst_ptr, max_bytes)
}

/// Copies triangle indices (`uint32` per index) directly into destination pointer.
#[pyfunction]
#[pyo3(signature = (indices_buffer, dst_ptr, max_bytes=None))]
pub fn copy_indices_to_ptr(
    indices_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    copy_buffer_to_ptr(indices_buffer, dst_ptr, max_bytes)
}

/// Copies quad indices (`uint32` per index) directly into destination pointer.
#[pyfunction]
#[pyo3(signature = (quad_indices_buffer, dst_ptr, max_bytes=None))]
pub fn copy_quad_indices_to_ptr(
    quad_indices_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    copy_buffer_to_ptr(quad_indices_buffer, dst_ptr, max_bytes)
}

/// Copies loop UV coordinates (`float32 * 2` per loop) directly into destination pointer.
#[pyfunction]
#[pyo3(signature = (loop_uvs_buffer, dst_ptr, max_bytes=None))]
pub fn copy_loop_uvs_to_ptr(
    loop_uvs_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    copy_buffer_to_ptr(loop_uvs_buffer, dst_ptr, max_bytes)
}

/// Copies polygon loop start offsets (`int32` per polygon) into destination pointer.
#[pyfunction]
#[pyo3(signature = (poly_count, is_quad, dst_ptr, max_bytes=None))]
pub fn copy_loop_starts_to_ptr(
    poly_count: usize,
    is_quad: bool,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    if poly_count == 0 {
        return Ok(0);
    }
    let required_bytes = poly_count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, required_bytes, max_bytes)?;

    let stride = if is_quad { 4i32 } else { 3i32 };
    let dst = dst_ptr as *mut i32;
    for i in 0..poly_count {
        unsafe {
            *dst.add(i) = (i as i32) * stride;
        }
    }
    Ok(required_bytes)
}

/// Copies polygon loop totals (`int32` per polygon) into destination pointer.
#[pyfunction]
#[pyo3(signature = (poly_count, is_quad, dst_ptr, max_bytes=None))]
pub fn copy_loop_totals_to_ptr(
    poly_count: usize,
    is_quad: bool,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    if poly_count == 0 {
        return Ok(0);
    }
    let required_bytes = poly_count * std::mem::size_of::<i32>();
    validate_pointer_and_bounds(dst_ptr, required_bytes, max_bytes)?;

    let stride = if is_quad { 4i32 } else { 3i32 };
    let dst = dst_ptr as *mut i32;
    for i in 0..poly_count {
        unsafe {
            *dst.add(i) = stride;
        }
    }
    Ok(required_bytes)
}

/// Copies face materials into destination pointer, widening `uint16` to Blender's `int32` material slot format.
#[pyfunction]
#[pyo3(signature = (face_materials_buffer, dst_ptr, max_bytes=None))]
pub fn copy_face_materials_to_ptr(
    face_materials_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    let py_buf: PyBuffer<u8> = PyBuffer::get(face_materials_buffer)?;
    let raw_bytes =
        unsafe { std::slice::from_raw_parts(py_buf.buf_ptr() as *const u8, py_buf.len_bytes()) };
    if raw_bytes.is_empty() {
        return Ok(0);
    }

    // If source buffer is u16 (2 bytes per face), expand to i32 (4 bytes per face)
    if raw_bytes.len() % 2 == 0 && py_buf.item_size() <= 2 {
        let count = raw_bytes.len() / 2;
        let required_bytes = count * std::mem::size_of::<i32>();
        validate_pointer_and_bounds(dst_ptr, required_bytes, max_bytes)?;

        let dst = dst_ptr as *mut i32;
        let src_u16 = unsafe { std::slice::from_raw_parts(py_buf.buf_ptr() as *const u16, count) };
        for (i, &mat) in src_u16.iter().enumerate() {
            unsafe {
                *dst.add(i) = mat as i32;
            }
        }
        Ok(required_bytes)
    } else {
        // Already 4 bytes per item (int32 / uint32)
        copy_buffer_to_ptr(face_materials_buffer, dst_ptr, max_bytes)
    }
}

/// Copies vertex RGBA colors (`float32 * 4` per vertex) directly into destination pointer.
#[pyfunction]
#[pyo3(signature = (colors_buffer, dst_ptr, max_bytes=None))]
pub fn copy_colors_to_ptr(
    colors_buffer: &Bound<'_, PyAny>,
    dst_ptr: usize,
    max_bytes: Option<usize>,
) -> PyResult<usize> {
    copy_buffer_to_ptr(colors_buffer, dst_ptr, max_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pointer_and_bounds() {
        // Null pointer check
        assert!(validate_pointer_and_bounds(0, 100, None).is_err());

        // Bounds limit check
        assert!(validate_pointer_and_bounds(0x1000, 100, Some(50)).is_err());
        assert!(validate_pointer_and_bounds(0x1000, 100, Some(100)).is_ok());
        assert!(validate_pointer_and_bounds(0x1000, 100, Some(200)).is_ok());
    }
}
