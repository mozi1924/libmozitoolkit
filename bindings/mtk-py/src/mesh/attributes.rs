//! Attribute access and management for `MeshData`.

use pyo3::buffer::PyBuffer;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyMemoryView};

use super::PyMeshData;

/// Read-only zero-copy memoryview of a numeric custom attribute by name.
pub fn attribute_memoryview<'py>(
    mesh: &PyMeshData,
    py: Python<'py>,
    name: &str,
) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
    if let Some(attr) = mesh.inner.get_custom_attribute(name) {
        if let Some(bytes_slice) = attr.as_bytes() {
            let bytes = PyBytes::new(py, bytes_slice);
            return Ok(Some(PyMemoryView::from(&bytes)?));
        }
    }
    Ok(None)
}

/// Add a numeric custom attribute from any Python object supporting the Buffer Protocol.
pub fn add_attribute_from_buffer(
    mesh: &mut PyMeshData,
    name: &str,
    domain: &str,
    dtype: &str,
    buffer: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let domain_enum = crate::attributes::parse_domain_str(domain)?;

    let py_buf: PyBuffer<u8> = PyBuffer::get(buffer)?;
    let raw_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(py_buf.buf_ptr() as *const u8, py_buf.len_bytes()) };

    let attr_data = crate::attributes::parse_attribute_buffer(dtype, raw_bytes)?;

    mesh.inner
        .add_custom_attribute(mtk_core::attributes::MeshAttribute {
            name: name.to_string(),
            domain: domain_enum,
            data: attr_data,
        });

    Ok(())
}

/// Add a string custom attribute.
pub fn add_string_attribute(
    mesh: &mut PyMeshData,
    name: &str,
    domain: &str,
    values: Vec<String>,
) -> PyResult<()> {
    let domain_enum = crate::attributes::parse_domain_str(domain)?;

    mesh.inner
        .add_custom_attribute(mtk_core::attributes::MeshAttribute {
            name: name.to_string(),
            domain: domain_enum,
            data: mtk_core::attributes::AttributeData::String(values),
        });

    Ok(())
}

/// Retrieve a string custom attribute's values.
pub fn get_string_attribute(mesh: &PyMeshData, name: &str) -> Option<Vec<String>> {
    if let Some(attr) = mesh.inner.get_custom_attribute(name) {
        if let mtk_core::attributes::AttributeData::String(ref vals) = attr.data {
            return Some(vals.clone());
        }
    }
    None
}

/// Check if a custom attribute exists.
pub fn has_attribute(mesh: &PyMeshData, name: &str) -> bool {
    mesh.inner.has_custom_attribute(name)
}

/// Remove a custom attribute by name. Returns True if existed and removed.
pub fn remove_attribute(mesh: &mut PyMeshData, name: &str) -> bool {
    mesh.inner.remove_custom_attribute(name).is_some()
}

/// List all custom attribute names.
pub fn attribute_names(mesh: &PyMeshData) -> Vec<String> {
    mesh.inner.custom_attribute_names()
}

/// Get attribute metadata: (domain_name, type_name, element_count).
pub fn attribute_info(mesh: &PyMeshData, name: &str) -> Option<(String, String, usize)> {
    mesh.inner.get_custom_attribute(name).map(|attr| {
        (
            attr.domain.as_str().to_string(),
            attr.data.type_name().to_string(),
            attr.len(),
        )
    })
}

/// Retrieve attribute data as a native Python list.
pub fn get_attribute_data<'py>(
    mesh: &PyMeshData,
    py: Python<'py>,
    name: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    if let Some(attr) = mesh.inner.get_custom_attribute(name) {
        crate::attributes::attribute_data_to_py_list(py, &attr.data).map(Some)
    } else {
        Ok(None)
    }
}

// -------------------------------------------------------------------------
// Flattened and list-based property helpers
// -------------------------------------------------------------------------

/// Flattened vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
pub fn get_flat_positions<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, mesh.inner.positions_flat())
}

/// Flattened vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
pub fn get_flat_normals<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, mesh.inner.normals_flat())
}

/// Flattened vertex UVs `[u0, v0, u1, v1, ...]`.
pub fn get_flat_uvs<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, mesh.inner.uvs_flat())
}

/// Triangle face indices `[i0, i1, i2, i3, i4, i5, ...]`.
pub fn get_indices<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, &mesh.inner.indices)
}

/// Quad polygon vertex indices `[v0, v1, v2, v3, ...]` (4 u32 per quad).
pub fn get_quad_indices<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    let quads = mesh.inner.reconstruct_quad_indices();
    PyList::new(py, &quads)
}

/// Number of quad faces recorded.
pub fn quad_count(mesh: &PyMeshData) -> usize {
    if let Some(ref quads) = mesh.inner.quad_indices {
        quads.len() / 4
    } else {
        mesh.inner.indices.len() / 6
    }
}

/// Material slot per face.
pub fn get_face_materials<'py>(mesh: &PyMeshData, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, &mesh.inner.face_materials)
}

/// Tint index per face.
pub fn get_face_tint_indices<'py>(
    mesh: &PyMeshData,
    py: Python<'py>,
) -> PyResult<Bound<'py, PyList>> {
    PyList::new(py, &mesh.inner.face_tint_indices)
}
