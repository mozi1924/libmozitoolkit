//! Standalone Blender raw-memory adapter wheel.
//!
//! The module knows only typed source buffers and destination addresses. It has
//! no dependency on `libmtk_py`, `mtk-core`, or `mtk-voxel`.

use std::mem::{align_of, size_of};

use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Integer API version for the standalone DNA adapter contract.
pub const API_VERSION: u32 = 1;

fn map_memory_err(err: mtk_blender_dna::DnaError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

macro_rules! impl_copy_fn {
    ($name:ident, $ty:ty, $copy_fn:path) => {
        #[pyfunction]
        #[pyo3(signature = (src, dst_ptr, count, max_bytes=None))]
        pub fn $name(
            src: &Bound<'_, PyAny>,
            dst_ptr: usize,
            count: usize,
            max_bytes: Option<usize>,
        ) -> PyResult<usize> {
            if count == 0 {
                return Ok(0);
            }

            // Fast path: a typed, C-contiguous buffer (NumPy / typed memoryview).
            if let Ok(buf) = PyBuffer::<$ty>::get(src) {
                if buf.is_c_contiguous() && buf.item_count() == count {
                    let slice =
                        unsafe { std::slice::from_raw_parts(buf.buf_ptr() as *const $ty, count) };
                    return unsafe { $copy_fn(slice, dst_ptr, max_bytes) }.map_err(map_memory_err);
                }
            }

            // Fallback: raw bytes view (`memoryview(...).cast("B")` / bytes).
            let buf = PyBuffer::<u8>::get(src).map_err(|_| {
                PyValueError::new_err(format!(
                    "source buffer is not a compatible {} array of {} element(s)",
                    stringify!($ty),
                    count
                ))
            })?;
            let expected = count * size_of::<$ty>();
            if buf.len_bytes() != expected {
                return Err(PyValueError::new_err(format!(
                    "source buffer is {} bytes, expected {} bytes for {} {} element(s)",
                    buf.len_bytes(),
                    expected,
                    count,
                    stringify!($ty)
                )));
            }
            if !buf.is_c_contiguous() {
                return Err(PyValueError::new_err("source buffer must be C-contiguous"));
            }
            let raw_ptr = buf.buf_ptr() as usize;
            if raw_ptr % align_of::<$ty>() != 0 {
                return Err(PyValueError::new_err(format!(
                    "source buffer pointer 0x{raw_ptr:x} is not aligned to {} bytes",
                    align_of::<$ty>()
                )));
            }
            let slice = unsafe { std::slice::from_raw_parts(raw_ptr as *const $ty, count) };
            unsafe { $copy_fn(slice, dst_ptr, max_bytes) }.map_err(map_memory_err)
        }
    };
}

impl_copy_fn!(copy_f32, f32, mtk_blender_dna::copy_f32_to_ptr);
impl_copy_fn!(copy_i32, i32, mtk_blender_dna::copy_i32_to_ptr);
impl_copy_fn!(copy_u32, u32, mtk_blender_dna::copy_u32_to_ptr);
impl_copy_fn!(copy_u8, u8, mtk_blender_dna::copy_u8_to_ptr);

/// Returns the standalone adapter API version.
#[pyfunction]
pub fn api_version() -> u32 {
    API_VERSION
}

#[pymodule]
fn libmtk_blender_dna(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(copy_f32, m)?)?;
    m.add_function(wrap_pyfunction!(copy_i32, m)?)?;
    m.add_function(wrap_pyfunction!(copy_u32, m)?)?;
    m.add_function(wrap_pyfunction!(copy_u8, m)?)?;
    m.add_function(wrap_pyfunction!(api_version, m)?)?;
    Ok(())
}
