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

            // Direct Fast Path: raw bytes buffer view (PyBufferOwner, memoryview, bytes)
            if let Ok(buf) = PyBuffer::<u8>::get(src) {
                let expected = count * size_of::<$ty>();
                if buf.is_c_contiguous() && buf.len_bytes() == expected {
                    let raw_ptr = buf.buf_ptr() as usize;
                    if raw_ptr % align_of::<$ty>() == 0 {
                        let slice =
                            unsafe { std::slice::from_raw_parts(raw_ptr as *const $ty, count) };
                        return unsafe { $copy_fn(slice, dst_ptr, max_bytes) }
                            .map_err(map_memory_err);
                    }
                }
            }

            // Fast Path for typed buffers (NumPy arrays or typed memoryviews)
            if let Ok(buf) = PyBuffer::<$ty>::get(src) {
                if buf.is_c_contiguous() && buf.item_count() == count {
                    let slice =
                        unsafe { std::slice::from_raw_parts(buf.buf_ptr() as *const $ty, count) };
                    return unsafe { $copy_fn(slice, dst_ptr, max_bytes) }.map_err(map_memory_err);
                }
            }

            Err(PyValueError::new_err(format!(
                "source buffer is not a compatible C-contiguous aligned {} array of {} element(s)",
                stringify!($ty),
                count
            )))
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
