//! # `mtk-py` Mesh Buffer Binding
//!
//! Exposes contiguous geometry buffers (`MeshData`) to Python with zero-copy
//! buffer protocol and memoryview support for ultrafast Blender vertex injection.

use std::collections::HashMap;

use pyo3::buffer::PyBuffer;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyMemoryView};

use mtk_core::attributes::{FaceAttributes, MaterialSlotId, TintIndex};
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::mesh::MeshData;

/// Supported attribute domains matching modern DCC & OpenUSD standards.
#[pyclass(name = "AttributeDomain", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyAttributeDomain {
    Point,
    Corner,
    Face,
    Mesh,
}

/// Python-facing wrapper around contiguous `MeshData`.
#[pyclass(name = "MeshData")]
#[derive(Debug, Clone, Default)]
pub struct PyMeshData {
    pub inner: MeshData,
}

impl PyMeshData {
    /// Direct reference to underlying MeshData.
    pub fn inner(&self) -> &MeshData {
        &self.inner
    }

    /// Direct mutable reference to underlying MeshData.
    pub fn inner_mut(&mut self) -> &mut MeshData {
        &mut self.inner
    }
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

    /// Sets raw vertex and face buffers from Python lists.
    #[pyo3(signature = (positions, normals, uvs, indices, face_materials=None, face_tint_indices=None))]
    pub fn set_buffers(
        &mut self,
        positions: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
        face_materials: Option<Vec<MaterialSlotId>>,
        face_tint_indices: Option<Vec<TintIndex>>,
    ) {
        let fc = indices.len() / 6;
        self.inner.positions = positions;
        self.inner.normals = normals;
        self.inner.uvs = uvs;
        self.inner.indices = indices;
        self.inner.face_materials = face_materials.unwrap_or_else(|| vec![0; fc]);
        self.inner.face_tint_indices = face_tint_indices.unwrap_or_else(|| vec![-1; fc]);
    }


    // -------------------------------------------------------------------------
    // Zero-Copy MemoryView Exports (for Blender `foreach_set` & NumPy `frombuffer`)
    // -------------------------------------------------------------------------

    /// Read-only memoryview of vertex positions as raw bytes (`float32 * 3` per vertex).
    ///
    /// Shape: `(vertex_count * 3,)` float32 or `(vertex_count, 3)` in NumPy via `np.frombuffer(m.positions_memoryview(), dtype=np.float32)`.
    pub fn positions_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.positions.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
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
        if self.inner.normals.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
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
        if self.inner.uvs.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
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
    pub fn get_flat_secondary_uvs<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyList>>> {
        if let Some(ref sec_uvs) = self.inner.secondary_uvs {
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
        if positions.len() % 3 != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "positions buffer length must be a multiple of 3",
            ));
        }
        if uvs.len() % 2 != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "uvs buffer length must be a multiple of 2",
            ));
        }
        let v_count = positions.len() / 3;
        let uv_count = uvs.len() / 2;
        if v_count != uv_count {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Vertex count ({}) must match UV count ({})",
                v_count, uv_count
            )));
        }
        if indices.len() % 3 != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "indices buffer length must be a multiple of 3",
            ));
        }
        for &idx in &indices {
            if idx as usize >= v_count {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Index {} out of bounds for vertex count {}",
                    idx, v_count
                )));
            }
        }
        if let Some(ref norms) = normals {
            if norms.len() != positions.len() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Normals count ({}) must match positions count ({})",
                    norms.len(),
                    positions.len()
                )));
            }
        }
        let tri_faces = indices.len() / 3;
        let quad_faces = indices.len() / 6;
        if let Some(ref mats) = face_materials {
            if mats.len() != tri_faces && mats.len() != quad_faces && !(indices.is_empty() && mats.is_empty()) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "face_materials length ({}) must match triangle count ({}) or quad count ({})",
                    mats.len(),
                    tri_faces,
                    quad_faces
                )));
            }
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
        let tints = vec![-1; mats.len()];

        Ok(Self {
            inner: MeshData {
                positions: pos_vec,
                normals: norm_vec,
                uvs: uv_vec,
                secondary_uvs: None,
                colors: None,
                indices,
                quad_indices: None,
                face_materials: mats,
                face_tint_indices: tints,
                custom_attributes: HashMap::new(),
            },
        })
    }

    /// Read-only memoryview of triangle indices as raw bytes (`uint32` per index).
    pub fn indices_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.indices.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.indices.as_ptr() as *const u8,
                self.inner.indices.len() * std::mem::size_of::<u32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of quad indices as raw bytes (`uint32` per index) if quads are recorded.
    pub fn quad_indices_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(ref quads) = self.inner.quad_indices {
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
    pub fn loop_uvs_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let is_quad = self.inner.quad_indices.is_some()
            || (!self.inner.indices.is_empty()
                && self.inner.indices.len() % 6 == 0
                && self.inner.face_materials.len() == self.inner.indices.len() / 6);

        let total_loops = if is_quad {
            if let Some(ref quads) = self.inner.quad_indices {
                quads.len()
            } else {
                (self.inner.indices.len() / 6) * 4
            }
        } else {
            self.inner.indices.len()
        };

        if total_loops == 0 {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }

        // When self.inner.uvs already stores per-corner (loop) UVs (e.g. from SectionMesher,
        // append_quad, or where spatial welding preserved corner UVs), its length matches total_loops.
        // Return a zero-copy memoryview directly over uvs_flat() without re-indexing!
        if self.inner.uvs.len() == total_loops {
            let uvs_flat = self.inner.uvs_flat();
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
                let quads = self.inner.reconstruct_quad_indices();
                let mut uvs = Vec::with_capacity(quads.len() * 2);
                for &v_idx in &quads {
                    if let Some(uv) = self.inner.uvs.get(v_idx as usize) {
                        uvs.push(uv[0]);
                        uvs.push(uv[1]);
                    } else {
                        uvs.push(0.0);
                        uvs.push(0.0);
                    }
                }
                uvs
            } else if !self.inner.indices.is_empty() {
                let mut uvs = Vec::with_capacity(self.inner.indices.len() * 2);
                for &v_idx in &self.inner.indices {
                    if let Some(uv) = self.inner.uvs.get(v_idx as usize) {
                        uvs.push(uv[0]);
                        uvs.push(uv[1]);
                    } else {
                        uvs.push(0.0);
                        uvs.push(0.0);
                    }
                }
                uvs
            } else {
                let mut uvs = Vec::with_capacity(self.inner.uvs.len() * 2);
                for uv in &self.inner.uvs {
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
    pub fn loop_starts_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let is_quad = self.inner.quad_indices.is_some()
            || (!self.inner.indices.is_empty()
                && self.inner.indices.len() % 6 == 0
                && self.inner.face_materials.len() == self.inner.indices.len() / 6);
        let count = if is_quad {
            if let Some(ref quads) = self.inner.quad_indices {
                quads.len() / 4
            } else {
                self.inner.indices.len() / 6
            }
        } else {
            self.inner.indices.len() / 3
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
    pub fn loop_totals_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let is_quad = self.inner.quad_indices.is_some()
            || (!self.inner.indices.is_empty()
                && self.inner.indices.len() % 6 == 0
                && self.inner.face_materials.len() == self.inner.indices.len() / 6);
        let count = if is_quad {
            if let Some(ref quads) = self.inner.quad_indices {
                quads.len() / 4
            } else {
                self.inner.indices.len() / 6
            }
        } else {
            self.inner.indices.len() / 3
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
    pub fn face_materials_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.face_materials.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
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
        if self.inner.face_tint_indices.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
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

    // -------------------------------------------------------------------------
    // Generic Zero-Copy Attribute System (for Blender / NumPy / WebGPU)
    // -------------------------------------------------------------------------

    /// Read-only zero-copy memoryview of a numeric custom attribute by name.
    /// Returns None if the attribute doesn't exist or is a non-POD type (e.g. String).
    pub fn attribute_memoryview<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        if let Some(attr) = self.inner.get_custom_attribute(name) {
            if let Some(bytes_slice) = attr.as_bytes() {
                let bytes = PyBytes::new(py, bytes_slice);
                return Ok(Some(PyMemoryView::from(&bytes)?));
            }
        }
        Ok(None)
    }

    /// Add a numeric custom attribute from any Python object supporting the Buffer Protocol
    /// (e.g. `bytes`, `bytearray`, `memoryview`, `numpy.ndarray`).
    ///
    /// Args:
    ///     name: Attribute identifier (e.g. "mtk_atlas_chunk_id", "mtk_uv_mode")
    ///     domain: Attribute domain ("point", "corner", "face", "mesh")
    ///     dtype: Data type ("float", "float2", "float3", "float4", "int8", "int16", "int32", "uint8", "uint16", "uint32", "bool")
    ///     buffer: Python buffer containing raw contiguous bytes
    pub fn add_attribute_from_buffer(
        &mut self,
        name: &str,
        domain: &str,
        dtype: &str,
        buffer: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let domain_enum = match domain.trim().to_lowercase().as_str() {
            "point" | "vertex" => mtk_core::attributes::AttributeDomain::Point,
            "corner" | "loop" | "face_varying" => mtk_core::attributes::AttributeDomain::Corner,
            "face" | "polygon" | "uniform" => mtk_core::attributes::AttributeDomain::Face,
            "mesh" | "global" | "constant" => mtk_core::attributes::AttributeDomain::Mesh,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid attribute domain '{}'. Expected 'point', 'corner', 'face', or 'mesh'",
                    domain
                )))
            }
        };

        let py_buf: PyBuffer<u8> = PyBuffer::get(buffer)?;
        let raw_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(py_buf.buf_ptr() as *const u8, py_buf.len_bytes())
        };

        let attr_data = match dtype.trim().to_lowercase().as_str() {
            "float" | "float32" | "f32" => {
                let elem_size = std::mem::size_of::<f32>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(float)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Float(vec)
            }
            "float2" | "vec2" => {
                let elem_size = std::mem::size_of::<[f32; 2]>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(float2)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Float2(vec)
            }
            "float3" | "vec3" => {
                let elem_size = std::mem::size_of::<[f32; 3]>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(float3)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Float3(vec)
            }
            "float4" | "vec4" | "color" => {
                let elem_size = std::mem::size_of::<[f32; 4]>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(float4)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Float4(vec)
            }
            "int8" | "i8" => {
                let count = raw_bytes.len();
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Int8(vec)
            }
            "int16" | "i16" => {
                let elem_size = std::mem::size_of::<i16>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(int16)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Int16(vec)
            }
            "int32" | "int" | "i32" => {
                let elem_size = std::mem::size_of::<i32>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(int32)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::Int32(vec)
            }
            "uint8" | "u8" | "byte" => {
                let count = raw_bytes.len();
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::UInt8(vec)
            }
            "uint16" | "u16" => {
                let elem_size = std::mem::size_of::<u16>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(uint16)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::UInt16(vec)
            }
            "uint32" | "u32" => {
                let elem_size = std::mem::size_of::<u32>();
                if raw_bytes.len() % elem_size != 0 {
                    return Err(pyo3::exceptions::PyValueError::new_err("Buffer byte length not divisible by sizeof(uint32)"));
                }
                let count = raw_bytes.len() / elem_size;
                let mut vec = Vec::with_capacity(count);
                if count > 0 {
                    unsafe {
                        std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr() as *mut u8, raw_bytes.len());
                        vec.set_len(count);
                    }
                }
                mtk_core::attributes::AttributeData::UInt32(vec)
            }
            "bool" | "boolean" => {
                let count = raw_bytes.len();
                let mut vec = Vec::with_capacity(count);
                for &b in raw_bytes {
                    vec.push(b != 0);
                }
                mtk_core::attributes::AttributeData::Bool(vec)
            }
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unsupported attribute data type '{}'",
                    dtype
                )))
            }
        };

        self.inner.add_custom_attribute(mtk_core::attributes::MeshAttribute {
            name: name.to_string(),
            domain: domain_enum,
            data: attr_data,
        });

        Ok(())
    }

    /// Add a string custom attribute (e.g. "mtk_source_texture_key").
    pub fn add_string_attribute(&mut self, name: &str, domain: &str, values: Vec<String>) -> PyResult<()> {
        let domain_enum = match domain.trim().to_lowercase().as_str() {
            "point" | "vertex" => mtk_core::attributes::AttributeDomain::Point,
            "corner" | "loop" | "face_varying" => mtk_core::attributes::AttributeDomain::Corner,
            "face" | "polygon" | "uniform" => mtk_core::attributes::AttributeDomain::Face,
            "mesh" | "global" | "constant" => mtk_core::attributes::AttributeDomain::Mesh,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid attribute domain '{}'",
                    domain
                )))
            }
        };

        self.inner.add_custom_attribute(mtk_core::attributes::MeshAttribute {
            name: name.to_string(),
            domain: domain_enum,
            data: mtk_core::attributes::AttributeData::String(values),
        });

        Ok(())
    }

    /// Retrieve a string custom attribute's values.
    pub fn get_string_attribute(&self, name: &str) -> Option<Vec<String>> {
        if let Some(attr) = self.inner.get_custom_attribute(name) {
            if let mtk_core::attributes::AttributeData::String(ref vals) = attr.data {
                return Some(vals.clone());
            }
        }
        None
    }

    /// Check if a custom attribute exists.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.inner.has_custom_attribute(name)
    }

    /// Remove a custom attribute by name. Returns True if existed and removed.
    pub fn remove_attribute(&mut self, name: &str) -> bool {
        self.inner.remove_custom_attribute(name).is_some()
    }

    /// List all custom attribute names.
    pub fn attribute_names(&self) -> Vec<String> {
        self.inner.custom_attribute_names()
    }

    /// Get attribute metadata: (domain_name, type_name, element_count).
    pub fn attribute_info(&self, name: &str) -> Option<(String, String, usize)> {
        self.inner.get_custom_attribute(name).map(|attr| {
            (
                attr.domain.as_str().to_string(),
                attr.data.type_name().to_string(),
                attr.len(),
            )
        })
    }

    /// Retrieve attribute data as a native Python list.
    pub fn get_attribute_data<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(attr) = self.inner.get_custom_attribute(name) {
            use mtk_core::attributes::AttributeData;
            match &attr.data {
                AttributeData::Float(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Float2(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Float3(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Float4(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Int8(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Int16(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Int32(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::UInt8(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::UInt16(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::UInt32(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::Bool(v) => Ok(Some(PyList::new(py, v)?.into_any())),
                AttributeData::String(v) => Ok(Some(PyList::new(py, v)?.into_any())),
            }
        } else {
            Ok(None)
        }
    }

    // -------------------------------------------------------------------------
    // Fallback Python List Helpers (for prototyping & debugging)
    // -------------------------------------------------------------------------

    /// Flattened vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    pub fn get_flat_positions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, self.inner.positions_flat())
    }

    /// Flattened vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    pub fn get_flat_normals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, self.inner.normals_flat())
    }

    /// Flattened vertex UVs `[u0, v0, u1, v1, ...]`.
    pub fn get_flat_uvs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, self.inner.uvs_flat())
    }

    /// Triangle face indices `[i0, i1, i2, i3, i4, i5, ...]`.
    pub fn get_indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, &self.inner.indices)
    }

    /// Quad polygon vertex indices `[v0, v1, v2, v3, ...]` (4 u32 per quad).
    pub fn get_quad_indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let quads = self.inner.reconstruct_quad_indices();
        PyList::new(py, &quads)
    }

    /// Number of quad faces recorded.
    #[getter]
    pub fn quad_count(&self) -> usize {
        if let Some(ref quads) = self.inner.quad_indices {
            quads.len() / 4
        } else {
            self.inner.indices.len() / 6
        }
    }

    /// Welds spatially duplicate vertices within tolerance while preserving indices and face attributes.
    #[pyo3(signature = (tolerance=1e-4))]
    pub fn weld_spatial_vertices(&mut self, tolerance: f32) {
        if !self.inner.is_empty() {
            self.inner.weld_spatial_vertices(tolerance);
        }
    }

    /// Culls duplicate overlapping faces and contacting interior coplanar faces from this mesh.
    ///
    /// Eliminates DCC viewport and render-time Z-fighting artifacts while preserving custom attributes.
    #[pyo3(signature = (tolerance=1e-3, cull_opposite=true, cull_duplicates=true))]
    pub fn cull_faces(&self, tolerance: f32, cull_opposite: bool, cull_duplicates: bool) -> PyMeshData {
        if self.inner.is_empty() {
            return PyMeshData::new();
        }
        let config = mtk_cull::MeshCullConfig {
            tolerance,
            cull_coplanar_opposite: cull_opposite,
            cull_duplicates,
        };
        let res = mtk_cull::cull_mesh_faces(&self.inner, &config);
        PyMeshData { inner: res.mesh }
    }

    /// Material slot per face.
    pub fn get_face_materials<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, &self.inner.face_materials)
    }

    /// Tint index per face.
    pub fn get_face_tint_indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        PyList::new(py, &self.inner.face_tint_indices)
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

pub use crate::extrude::{
    process_flat_mesh_extrude_repair, process_mesh_extrude_repair, process_random_extrude_mesh,
};
pub use crate::subdivide::{
    adaptive_pixel_split_mesh, calculate_face_target_grid, calculate_pixel_grid_cut_factors,
    slice_face_by_pixel_grid,
};
