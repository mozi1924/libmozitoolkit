//! # `mtk-py` (Python Bindings for libmozitoolkit)
//!
//! Exposes `libmtk` data structures and high-performance algorithms to Python via PyO3.

use pyo3::prelude::*;
use pyo3::types::PyList;

use mtk_core::mesh::MeshData;
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::attributes::FaceAttributes;

/// Python-facing wrapper around contiguous `MeshData`.
#[pyclass(name = "MeshData")]
#[derive(Debug, Clone)]
pub struct PyMeshData {
    pub(crate) inner: MeshData,
}

#[pymethods]
impl PyMeshData {
    #[new]
    fn new() -> Self {
        Self {
            inner: MeshData::new(),
        }
    }

    /// Number of vertices in the mesh.
    #[getter]
    fn vertex_count(&self) -> usize {
        self.inner.vertex_count()
    }

    /// Number of triangles in the mesh.
    #[getter]
    fn triangle_count(&self) -> usize {
        self.inner.triangle_count()
    }

    /// Number of faces recorded.
    #[getter]
    fn face_count(&self) -> usize {
        self.inner.face_count()
    }

    /// Checks if the mesh is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clears all geometry in the mesh.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Flattened vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    fn get_flat_positions<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let mut flat = Vec::with_capacity(self.inner.positions.len() * 3);
        for p in &self.inner.positions {
            flat.push(p[0]);
            flat.push(p[1]);
            flat.push(p[2]);
        }
        PyList::new(py, flat).expect("failed to create list")
    }

    /// Flattened vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    fn get_flat_normals<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let mut flat = Vec::with_capacity(self.inner.normals.len() * 3);
        for n in &self.inner.normals {
            flat.push(n[0]);
            flat.push(n[1]);
            flat.push(n[2]);
        }
        PyList::new(py, flat).expect("failed to create list")
    }

    /// Flattened vertex UVs `[u0, v0, u1, v1, ...]`.
    fn get_flat_uvs<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let mut flat = Vec::with_capacity(self.inner.uvs.len() * 2);
        for uv in &self.inner.uvs {
            flat.push(uv[0]);
            flat.push(uv[1]);
        }
        PyList::new(py, flat).expect("failed to create list")
    }

    /// Triangle face indices `[i0, i1, i2, i3, i4, i5, ...]`.
    fn get_indices<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, &self.inner.indices).expect("failed to create list")
    }

    /// Material slot per face.
    fn get_face_materials<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, &self.inner.face_materials).expect("failed to create list")
    }

    /// Appends a standard unit cube face in given direction (0: Down, 1: Up, 2: North, 3: South, 4: West, 5: East).
    fn append_unit_cube_face(&mut self, direction_id: u8, material_slot: u16) -> PyResult<()> {
        let dir = match direction_id {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::North,
            3 => Direction::South,
            4 => Direction::West,
            5 => Direction::East,
            _ => return Err(pyo3::exceptions::PyValueError::new_err("Invalid direction index (0..5 expected)")),
        };
        let quad = Quad::unit_cube_face(dir);
        let attr = FaceAttributes {
            material_slot,
            ..Default::default()
        };
        self.inner.append_quad(&quad, &attr);
        Ok(())
    }
}

/// Returns libmtk version string.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Main entrypoint for Python module `libmtk_py`.
#[pymodule]
fn libmtk_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMeshData>()?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_mesh_data_inner() {
        let mut mesh = PyMeshData::new();
        assert_eq!(mesh.vertex_count(), 0);
        assert!(mesh.is_empty());

        let res = mesh.append_unit_cube_face(1, 7);
        assert!(res.is_ok());
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.face_count(), 1);

        mesh.clear();
        assert_eq!(mesh.vertex_count(), 0);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_python_version_string() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}


