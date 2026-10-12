//! Safe zero-copy Python buffer protocol owner and memoryview factory.
//!
//! Exposes `PyBufferOwner` which implements Python's buffer protocol (`__getbuffer__`),
//! ensuring that parent Rust/Python structures or generated buffers remain alive without copying memory.

use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyMemoryView;
use std::os::raw::{c_int, c_void};

#[allow(dead_code)]
enum BufferStorage {
    Parent(Py<PyAny>),
    Bytes(Vec<u8>),
    F32(Vec<f32>),
    I32(Vec<i32>),
}

/// A lightweight Python buffer protocol adapter that provides a zero-copy view of
/// memory either borrowed from a parent Python object or held in typed Rust vectors.
#[pyclass]
pub struct PyBufferOwner {
    ptr: usize,
    len: usize,
    readonly: bool,
    _storage: BufferStorage,
}

#[pymethods]
impl PyBufferOwner {
    unsafe fn __getbuffer__(
        slf: &Bound<'_, Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        if view.is_null() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "view pointer is null",
            ));
        }
        let this = slf.borrow();
        if (flags & ffi::PyBUF_WRITABLE) != 0 && this.readonly {
            return Err(pyo3::exceptions::PyBufferError::new_err(
                "buffer is read-only",
            ));
        }

        (*view).buf = this.ptr as *mut c_void;
        (*view).obj = ffi::Py_NewRef(slf.as_ptr());
        (*view).len = this.len as ffi::Py_ssize_t;
        (*view).itemsize = 1;
        (*view).readonly = if this.readonly { 1 } else { 0 };
        (*view).ndim = 1;
        (*view).format = std::ptr::null_mut();
        (*view).shape = std::ptr::null_mut();
        (*view).strides = std::ptr::null_mut();
        (*view).suboffsets = std::ptr::null_mut();
        (*view).internal = std::ptr::null_mut();

        Ok(())
    }

    unsafe fn __releasebuffer__(&self, _view: *mut ffi::Py_buffer) {
        // Python buffer protocol decrements view->obj automatically.
    }
}

/// Creates a true zero-copy `PyMemoryView` referencing the slice at `ptr` with length `len`,
/// keeping `parent` alive in memory as long as the memoryview is referenced.
pub fn create_zerocopy_memoryview<'py>(
    py: Python<'py>,
    ptr: *const u8,
    len: usize,
    parent: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyMemoryView>> {
    if len == 0 {
        let bytes = pyo3::types::PyBytes::new(py, &[]);
        return PyMemoryView::from(&bytes);
    }
    let owner = Bound::new(
        py,
        PyBufferOwner {
            ptr: ptr as usize,
            len,
            readonly: true,
            _storage: BufferStorage::Parent(parent.clone().unbind()),
        },
    )?;
    unsafe {
        let py_mv_ptr = ffi::PyMemoryView_FromObject(owner.as_ptr());
        if py_mv_ptr.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(Bound::from_owned_ptr(py, py_mv_ptr).downcast_into::<PyMemoryView>()?)
    }
}

/// Creates a memoryview directly over an owned `Vec<u8>` without secondary `PyBytes` duplication.
pub fn create_owned_bytes_memoryview<'py>(
    py: Python<'py>,
    bytes: Vec<u8>,
) -> PyResult<Bound<'py, PyMemoryView>> {
    let len = bytes.len();
    if len == 0 {
        let empty = pyo3::types::PyBytes::new(py, &[]);
        return PyMemoryView::from(&empty);
    }
    let ptr = bytes.as_ptr() as usize;
    let owner = Bound::new(
        py,
        PyBufferOwner {
            ptr,
            len,
            readonly: true,
            _storage: BufferStorage::Bytes(bytes),
        },
    )?;
    unsafe {
        let py_mv_ptr = ffi::PyMemoryView_FromObject(owner.as_ptr());
        if py_mv_ptr.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(Bound::from_owned_ptr(py, py_mv_ptr).downcast_into::<PyMemoryView>()?)
    }
}

/// Creates a memoryview directly over an owned `Vec<f32>` without reallocating or unsafe alignment casting.
pub fn create_owned_f32_memoryview<'py>(
    py: Python<'py>,
    vec: Vec<f32>,
) -> PyResult<Bound<'py, PyMemoryView>> {
    let len = vec.len() * std::mem::size_of::<f32>();
    if len == 0 {
        let empty = pyo3::types::PyBytes::new(py, &[]);
        return PyMemoryView::from(&empty);
    }
    let ptr = vec.as_ptr() as usize;
    let owner = Bound::new(
        py,
        PyBufferOwner {
            ptr,
            len,
            readonly: true,
            _storage: BufferStorage::F32(vec),
        },
    )?;
    unsafe {
        let py_mv_ptr = ffi::PyMemoryView_FromObject(owner.as_ptr());
        if py_mv_ptr.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(Bound::from_owned_ptr(py, py_mv_ptr).downcast_into::<PyMemoryView>()?)
    }
}

/// Creates a memoryview directly over an owned `Vec<i32>` without reallocating or unsafe alignment casting.
pub fn create_owned_i32_memoryview<'py>(
    py: Python<'py>,
    vec: Vec<i32>,
) -> PyResult<Bound<'py, PyMemoryView>> {
    let len = vec.len() * std::mem::size_of::<i32>();
    if len == 0 {
        let empty = pyo3::types::PyBytes::new(py, &[]);
        return PyMemoryView::from(&empty);
    }
    let ptr = vec.as_ptr() as usize;
    let owner = Bound::new(
        py,
        PyBufferOwner {
            ptr,
            len,
            readonly: true,
            _storage: BufferStorage::I32(vec),
        },
    )?;
    unsafe {
        let py_mv_ptr = ffi::PyMemoryView_FromObject(owner.as_ptr());
        if py_mv_ptr.is_null() {
            return Err(PyErr::fetch(py));
        }
        Ok(Bound::from_owned_ptr(py, py_mv_ptr).downcast_into::<PyMemoryView>()?)
    }
}
