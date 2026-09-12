//! # `mtk-py` Mesh Buffer Binding
//!
//! Exposes contiguous geometry buffers (`MeshData`) to Python with zero-copy
//! buffer protocol and memoryview support for ultrafast Blender vertex injection.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyMemoryView};

use mtk_core::attributes::{FaceAttributes, MaterialSlotId, TintIndex};
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::mesh::MeshData;

/// Python-facing wrapper around contiguous `MeshData`.
#[pyclass(name = "MeshData")]
#[derive(Debug, Clone, Default)]
pub struct PyMeshData {
    pub(crate) inner: MeshData,
}

#[pymethods]
impl PyMeshData {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: MeshData::new(),
        }
    }

    /// Creates a new `MeshData` with pre-allocated capacity.
    #[staticmethod]
    pub fn with_capacity(num_vertices: usize, num_indices: usize, num_faces: usize) -> Self {
        Self {
            inner: MeshData::with_capacity(num_vertices, num_indices, num_faces),
        }
    }

    /// Number of vertices in the mesh buffer.
    #[getter]
    pub fn vertex_count(&self) -> usize {
        self.inner.vertex_count()
    }

    /// Number of triangles in the mesh buffer.
    #[getter]
    pub fn triangle_count(&self) -> usize {
        self.inner.triangle_count()
    }

    /// Number of quad faces recorded.
    #[getter]
    pub fn face_count(&self) -> usize {
        self.inner.face_count()
    }

    /// Checks if the mesh is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clears all geometry in the mesh.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Appends another `MeshData` into this mesh buffer.
    pub fn append_mesh(&mut self, other: &PyMeshData) {
        self.inner.append_mesh(&other.inner);
    }

    /// Appends a standard unit cube face in given direction (0: Down, 1: Up, 2: North, 3: South, 4: West, 5: East).
    #[pyo3(signature = (direction_id, material_slot=0, tint_index=-1))]
    pub fn append_unit_cube_face(
        &mut self,
        direction_id: u8,
        material_slot: MaterialSlotId,
        tint_index: TintIndex,
    ) -> PyResult<()> {
        let dir = match direction_id {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::North,
            3 => Direction::South,
            4 => Direction::West,
            5 => Direction::East,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid direction index (0..5 expected: 0=Down, 1=Up, 2=North, 3=South, 4=West, 5=East)",
                ))
            }
        };
        let quad = Quad::unit_cube_face(dir);
        let attr = FaceAttributes {
            material_slot,
            tint_index,
            ..Default::default()
        };
        self.inner.append_quad(&quad, &attr);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Zero-Copy MemoryView Exports (for Blender `foreach_set` & NumPy `frombuffer`)
    // -------------------------------------------------------------------------

    /// Read-only memoryview of vertex positions as raw bytes (`float32 * 3` per vertex).
    ///
    /// Shape: `(vertex_count * 3,)` float32 or `(vertex_count, 3)` in NumPy via `np.frombuffer(m.positions_memoryview(), dtype=np.float32)`.
    pub fn positions_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.positions.as_ptr() as *const u8,
                self.inner.positions.len() * std::mem::size_of::<[f32; 3]>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of vertex normals as raw bytes (`float32 * 3` per vertex).
    pub fn normals_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.normals.as_ptr() as *const u8,
                self.inner.normals.len() * std::mem::size_of::<[f32; 3]>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of vertex UV coordinates as raw bytes (`float32 * 2` per vertex).
    pub fn uvs_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.uvs.as_ptr() as *const u8,
                self.inner.uvs.len() * std::mem::size_of::<[f32; 2]>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of secondary normalized [0, 1] vertex UVs (if generated).
    pub fn secondary_uvs_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(ref sec_uvs) = self.inner.secondary_uvs {
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
    pub fn get_flat_secondary_uvs<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyList>> {
        if let Some(ref sec_uvs) = self.inner.secondary_uvs {
            let mut flat = Vec::with_capacity(sec_uvs.len() * 2);
            for uv in sec_uvs {
                flat.push(uv[0]);
                flat.push(uv[1]);
            }
            Some(PyList::new(py, &flat).expect("failed to create list"))
        } else {
            None
        }
    }

    /// Constructs a `MeshData` object directly from flat buffers.
    #[staticmethod]
    #[pyo3(signature = (positions, uvs, indices, normals=None, face_materials=None))]
    pub fn from_raw_buffers(
        positions: Vec<f32>,
        uvs: Vec<f32>,
        indices: Vec<u32>,
        normals: Option<Vec<f32>>,
        face_materials: Option<Vec<u16>>,
    ) -> PyResult<Self> {
        let v_count = positions.len() / 3;
        let uv_count = uvs.len() / 2;
        if v_count != uv_count {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Vertex count ({}) must match UV count ({})",
                v_count, uv_count
            )));
        }

        let mut pos_vec = Vec::with_capacity(v_count);
        for i in 0..v_count {
            pos_vec.push([positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]]);
        }

        let mut uv_vec = Vec::with_capacity(v_count);
        for i in 0..v_count {
            uv_vec.push([uvs[i * 2], uvs[i * 2 + 1]]);
        }

        let norm_vec = if let Some(norms) = normals {
            let n_count = norms.len() / 3;
            let mut nv = Vec::with_capacity(n_count);
            for i in 0..n_count {
                nv.push([norms[i * 3], norms[i * 3 + 1], norms[i * 3 + 2]]);
            }
            nv
        } else {
            vec![[0.0, 1.0, 0.0]; v_count]
        };

        let face_count = indices.len() / 3;
        let mats = face_materials.unwrap_or_else(|| vec![0; face_count]);
        let tints = vec![-1; face_count];

        Ok(Self {
            inner: MeshData {
                positions: pos_vec,
                normals: norm_vec,
                uvs: uv_vec,
                secondary_uvs: None,
                colors: None,
                indices,
                face_materials: mats,
                face_tint_indices: tints,
            },
        })
    }

    /// Read-only memoryview of triangle indices as raw bytes (`uint32` per index).
    pub fn indices_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.indices.as_ptr() as *const u8,
                self.inner.indices.len() * std::mem::size_of::<u32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of face material slots (`uint16` per face).
    pub fn face_materials_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.face_materials.as_ptr() as *const u8,
                self.inner.face_materials.len() * std::mem::size_of::<u16>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of face tint indices (`int8` per face).
    pub fn face_tint_indices_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.face_tint_indices.as_ptr() as *const u8,
                self.inner.face_tint_indices.len() * std::mem::size_of::<i8>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of vertex RGBA colors (`float32 * 4` per vertex) if present.
    pub fn colors_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(ref cols) = self.inner.colors {
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

    // -------------------------------------------------------------------------
    // Fallback Python List Helpers (for prototyping & debugging)
    // -------------------------------------------------------------------------

    /// Flattened vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    pub fn get_flat_positions<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, self.inner.positions_flat()).expect("failed to create list")
    }

    /// Flattened vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    pub fn get_flat_normals<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, self.inner.normals_flat()).expect("failed to create list")
    }

    /// Flattened vertex UVs `[u0, v0, u1, v1, ...]`.
    pub fn get_flat_uvs<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, self.inner.uvs_flat()).expect("failed to create list")
    }

    /// Triangle face indices `[i0, i1, i2, i3, i4, i5, ...]`.
    pub fn get_indices<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, &self.inner.indices).expect("failed to create list")
    }

    /// Material slot per face.
    pub fn get_face_materials<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, &self.inner.face_materials).expect("failed to create list")
    }

    /// Tint index per face.
    pub fn get_face_tint_indices<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        PyList::new(py, &self.inner.face_tint_indices).expect("failed to create list")
    }

    fn __repr__(&self) -> String {
        format!(
            "<MeshData vertices={} triangles={} faces={}>",
            self.vertex_count(),
            self.triangle_count(),
            self.face_count()
        )
    }
}
