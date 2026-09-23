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
                custom_attributes: HashMap::new(),
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const f32, vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const [f32; 2], vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const [f32; 3], vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const [f32; 4], vec.as_mut_ptr(), count);
                    vec.set_len(count);
                }
                mtk_core::attributes::AttributeData::Float4(vec)
            }
            "int8" | "i8" => {
                let count = raw_bytes.len();
                let mut vec = Vec::with_capacity(count);
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const i8, vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const i16, vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const i32, vec.as_mut_ptr(), count);
                    vec.set_len(count);
                }
                mtk_core::attributes::AttributeData::Int32(vec)
            }
            "uint8" | "u8" | "byte" => {
                let count = raw_bytes.len();
                let mut vec = Vec::with_capacity(count);
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr(), vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const u16, vec.as_mut_ptr(), count);
                    vec.set_len(count);
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
                unsafe {
                    std::ptr::copy_nonoverlapping(raw_bytes.as_ptr() as *const u32, vec.as_mut_ptr(), count);
                    vec.set_len(count);
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

    /// Quad polygon vertex indices `[v0, v1, v2, v3, ...]` (4 u32 per quad).
    pub fn get_quad_indices<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let quad_count = self.inner.indices.len() / 6;
        let mut quads = Vec::with_capacity(quad_count * 4);
        for q in 0..quad_count {
            let base = q * 6;
            quads.push(self.inner.indices[base]);
            quads.push(self.inner.indices[base + 1]);
            quads.push(self.inner.indices[base + 2]);
            quads.push(self.inner.indices[base + 5]);
        }
        PyList::new(py, &quads).expect("failed to create list")
    }

    /// Number of quad faces recorded.
    #[getter]
    pub fn quad_count(&self) -> usize {
        self.inner.indices.len() / 6
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

/// Calculates adaptive (cols, rows) subdivisions based on UV bounds and texture size.
#[pyfunction]
#[pyo3(signature = (uvs, tex_w, tex_h, pixels_per_face=1.0, max_subdivisions=64))]
pub fn calculate_face_target_grid(
    uvs: Vec<[f32; 2]>,
    tex_w: u32,
    tex_h: u32,
    pixels_per_face: f32,
    max_subdivisions: u32,
) -> (u32, u32) {
    mtk_core::subdivide::calculate_face_target_grid(
        &uvs,
        tex_w,
        tex_h,
        pixels_per_face,
        max_subdivisions,
    )
}

/// Performs adaptive pixel grid subdivision on a `PyMeshData` buffer.
#[pyfunction]
#[pyo3(signature = (mesh, face_resolutions=None, default_resolution=(16, 16), pixels_per_face=1.0, max_subdivisions=64, weld_dist=1e-4))]
pub fn adaptive_pixel_split_mesh(
    mesh: &PyMeshData,
    face_resolutions: Option<Vec<Option<(u32, u32)>>>,
    default_resolution: (u32, u32),
    pixels_per_face: f32,
    max_subdivisions: u32,
    weld_dist: f32,
) -> PyMeshData {
    let empty_vec = Vec::new();
    let res_slice = face_resolutions.as_deref().unwrap_or(&empty_vec);
    let output_mesh = mtk_core::subdivide::adaptive_pixel_split_mesh(
        &mesh.inner,
        res_slice,
        default_resolution,
        pixels_per_face,
        max_subdivisions,
        weld_dist,
    );
    PyMeshData { inner: output_mesh }
}

/// Performs high-performance batch Data In, Data Out UV repair across an entire mesh.
#[pyfunction]
#[pyo3(signature = (
    positions,
    face_vertices,
    face_uvs,
    face_materials,
    selected_faces,
    pixel_steps,
    uv_mode="SMART",
    repair_uv=true,
    add_crease=false,
    crease_val=1.0,
    only_collapsed=false
))]
pub fn process_mesh_extrude_repair(
    positions: Vec<[f32; 3]>,
    face_vertices: Vec<Vec<u32>>,
    face_uvs: Vec<Vec<[f32; 2]>>,
    face_materials: Vec<u32>,
    selected_faces: Vec<u32>,
    pixel_steps: Vec<[f32; 2]>,
    uv_mode: &str,
    repair_uv: bool,
    add_crease: bool,
    crease_val: f32,
    only_collapsed: bool,
) -> (
    Vec<(u32, Vec<[f32; 2]>)>,
    Vec<(u32, u32)>,
    Vec<((u32, u32), f32)>,
    usize,
) {
    let mode = match uv_mode.to_uppercase().as_str() {
        "INWARD" => mtk_core::extrude::ExtrudeUvMode::Inward,
        "OUTWARD" => mtk_core::extrude::ExtrudeUvMode::Outward,
        _ => mtk_core::extrude::ExtrudeUvMode::Smart,
    };

    let input = mtk_core::extrude_mesh::ExtrudeMeshInput {
        positions,
        face_vertices,
        face_uvs,
        face_materials,
        selected_faces,
        pixel_steps,
        config: mtk_core::extrude_mesh::MeshExtrudeRepairConfig {
            uv_mode: mode,
            repair_uv,
            add_crease,
            crease_val,
            only_collapsed,
        },
    };

    let out = mtk_core::extrude_mesh::process_mesh_extrude_repair(&input);
    (
        out.modified_face_uvs,
        out.modified_face_materials,
        out.modified_edge_creases,
        out.repaired_count,
    )
}

/// Performs complete batch discrete face extrusion, 3D noise vertex displacement, topology rebuilding,
/// and automatic side UV repair in a single batch pass (Data In, Data Out).
#[pyfunction]
#[pyo3(signature = (
    positions,
    face_vertices,
    face_uvs,
    face_materials,
    selected_faces,
    pixel_steps,
    min_height=0.0,
    max_height=0.1,
    seed=0,
    noise_type="RANDOM",
    noise_scale=1.0,
    repair_uv=true,
    uv_mode="SMART",
    add_crease=false,
    crease_val=1.0
))]
pub fn process_random_extrude_mesh(
    positions: Vec<[f32; 3]>,
    face_vertices: Vec<Vec<u32>>,
    face_uvs: Vec<Vec<[f32; 2]>>,
    face_materials: Vec<u32>,
    selected_faces: Vec<u32>,
    pixel_steps: Vec<[f32; 2]>,
    min_height: f32,
    max_height: f32,
    seed: u32,
    noise_type: &str,
    noise_scale: f32,
    repair_uv: bool,
    uv_mode: &str,
    add_crease: bool,
    crease_val: f32,
) -> (
    Vec<[f32; 3]>,
    Vec<Vec<u32>>,
    Vec<Vec<[f32; 2]>>,
    Vec<u32>,
    Vec<u32>,
    usize,
) {
    let n_type = match noise_type.to_uppercase().as_str() {
        "PERLIN" => mtk_core::extrude::ExtrudeNoiseType::Perlin,
        "CELL" | "CELLULAR" | "VORONOI" => mtk_core::extrude::ExtrudeNoiseType::Cellular,
        _ => mtk_core::extrude::ExtrudeNoiseType::UniformRandom,
    };

    let mode = match uv_mode.to_uppercase().as_str() {
        "INWARD" => mtk_core::extrude::ExtrudeUvMode::Inward,
        "OUTWARD" => mtk_core::extrude::ExtrudeUvMode::Outward,
        _ => mtk_core::extrude::ExtrudeUvMode::Smart,
    };

    let input = mtk_core::extrude_mesh::RandomExtrudeMeshInput {
        positions,
        face_vertices,
        face_uvs,
        face_materials,
        selected_faces,
        pixel_steps,
        min_height,
        max_height,
        seed,
        noise_type: n_type,
        noise_scale,
        repair_uv,
        uv_mode: mode,
        add_crease,
        crease_val,
    };

    let out = mtk_core::extrude_mesh::process_random_extrude_mesh(&input);
    (
        out.new_positions,
        out.new_face_vertices,
        out.new_face_uvs,
        out.new_face_materials,
        out.extruded_face_indices,
        out.repaired_count,
    )
}

