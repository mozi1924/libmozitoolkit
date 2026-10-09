//! # `mtk-py` Mesh Buffer Binding
//!
//! Exposes contiguous geometry buffers (`MeshData`) to Python with zero-copy
//! buffer protocol and memoryview support for ultrafast Blender vertex injection.

pub mod attributes;
pub mod direct_ptr;
pub mod memoryview;

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyMemoryView};

use mtk_core::attributes::{FaceAttributes, MaterialSlotId, TintIndex};
use mtk_core::direction::Direction;
use mtk_core::geometry::Quad;
use mtk_core::mesh::MeshData;

pub use crate::attributes::PyAttributeDomain;

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
            if mats.len() != tri_faces
                && mats.len() != quad_faces
                && !(indices.is_empty() && mats.is_empty())
            {
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
    pub fn cull_faces(
        &self,
        tolerance: f32,
        cull_opposite: bool,
        cull_duplicates: bool,
    ) -> PyMeshData {
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

    // -------------------------------------------------------------------------
    // Zero-Copy MemoryView Delegates (implemented in memoryview.rs)
    // -------------------------------------------------------------------------

    /// Read-only memoryview of vertex positions as raw bytes (`float32 * 3` per vertex).
    pub fn positions_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::positions_memoryview(self, py)
    }

    /// Read-only memoryview of vertex normals as raw bytes (`float32 * 3` per vertex).
    pub fn normals_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::normals_memoryview(self, py)
    }

    /// Read-only memoryview of vertex UV coordinates as raw bytes (`float32 * 2` per vertex).
    pub fn uvs_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::uvs_memoryview(self, py)
    }

    /// Read-only memoryview of secondary normalized [0, 1] vertex UVs (if generated).
    pub fn secondary_uvs_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        memoryview::secondary_uvs_memoryview(self, py)
    }

    /// Flattened secondary vertex UVs `[u0, v0, u1, v1, ...]`.
    pub fn get_flat_secondary_uvs<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyList>>> {
        memoryview::get_flat_secondary_uvs(self, py)
    }

    /// Read-only memoryview of triangle indices as raw bytes (`uint32` per index).
    pub fn indices_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::indices_memoryview(self, py)
    }

    /// Read-only memoryview of quad indices as raw bytes (`uint32` per index) if quads are recorded.
    pub fn quad_indices_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        memoryview::quad_indices_memoryview(self, py)
    }

    /// Read-only memoryview of expanded per-loop UV coordinates (`float32 * 2` per loop).
    pub fn loop_uvs_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::loop_uvs_memoryview(self, py)
    }

    /// Read-only memoryview of polygon loop start offsets (`int32` per polygon).
    pub fn loop_starts_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::loop_starts_memoryview(self, py)
    }

    /// Read-only memoryview of polygon loop totals (`int32` per polygon: 4 for quads, 3 for triangles).
    pub fn loop_totals_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::loop_totals_memoryview(self, py)
    }

    /// Read-only memoryview of face material slots (`uint16` per face).
    pub fn face_materials_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::face_materials_memoryview(self, py)
    }

    /// Read-only memoryview of face tint indices (`int8` per face).
    pub fn face_tint_indices_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::face_tint_indices_memoryview(self, py)
    }

    /// Read-only memoryview of vertex RGBA colors (`float32 * 4` per vertex) if present.
    pub fn colors_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        memoryview::colors_memoryview(self, py)
    }

    /// Returns a list of unique material slot IDs present across all faces in this mesh.
    pub fn used_materials(&self) -> Vec<u16> {
        self.inner.used_materials()
    }

    /// Compacts face material slot IDs into contiguous 0..N-1 and returns original mapping.
    pub fn compact_materials(&mut self) -> Vec<u16> {
        self.inner.compact_materials()
    }

    // -------------------------------------------------------------------------
    // Direct Pointer Memory Transfer Delegates (implemented in direct_ptr.rs)
    // -------------------------------------------------------------------------

    /// Copies vertex positions directly into the destination host pointer (`float32 * 3` per vertex).
    pub fn direct_copy_positions_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_positions_to_ptr(self, dst_ptr)
    }

    /// Copies vertex normals directly into the destination host pointer (`float32 * 3` per vertex).
    pub fn direct_copy_normals_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_normals_to_ptr(self, dst_ptr)
    }

    /// Copies triangle indices directly into the destination host pointer (`uint32` per index).
    pub fn direct_copy_indices_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_indices_to_ptr(self, dst_ptr)
    }

    /// Copies reconstructed quad indices directly into the destination host pointer (`uint32` per index).
    pub fn direct_copy_quad_indices_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_quad_indices_to_ptr(self, dst_ptr)
    }

    /// Copies expanded per-loop UVs directly into the destination host pointer (`float32 * 2` per loop).
    pub fn direct_copy_loop_uvs_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_loop_uvs_to_ptr(self, dst_ptr)
    }

    /// Copies polygon loop start offsets directly into the destination host pointer (`int32` per polygon).
    pub fn direct_copy_loop_starts_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_loop_starts_to_ptr(self, dst_ptr)
    }

    /// Copies polygon loop totals directly into the destination host pointer (`int32` per polygon).
    pub fn direct_copy_loop_totals_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_loop_totals_to_ptr(self, dst_ptr)
    }

    /// Copies face material slots converted to 32-bit integers directly into the destination host pointer.
    pub fn direct_copy_face_materials_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_face_materials_to_ptr(self, dst_ptr)
    }

    /// Copies vertex RGBA colors directly into the destination host pointer (`float32 * 4` per vertex).
    pub fn direct_copy_colors_to_ptr(&self, dst_ptr: usize) -> PyResult<usize> {
        direct_ptr::direct_copy_colors_to_ptr(self, dst_ptr)
    }

    // -------------------------------------------------------------------------
    // Custom Attributes & List Delegates (implemented in attributes.rs)
    // -------------------------------------------------------------------------

    /// Read-only zero-copy memoryview of a numeric custom attribute by name.
    pub fn attribute_memoryview<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        attributes::attribute_memoryview(self, py, name)
    }

    /// Add a numeric custom attribute from any Python buffer.
    pub fn add_attribute_from_buffer(
        &mut self,
        name: &str,
        domain: &str,
        dtype: &str,
        buffer: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        attributes::add_attribute_from_buffer(self, name, domain, dtype, buffer)
    }

    /// Add a string custom attribute.
    pub fn add_string_attribute(
        &mut self,
        name: &str,
        domain: &str,
        values: Vec<String>,
    ) -> PyResult<()> {
        attributes::add_string_attribute(self, name, domain, values)
    }

    /// Retrieve a string custom attribute's values.
    pub fn get_string_attribute(&self, name: &str) -> Option<Vec<String>> {
        attributes::get_string_attribute(self, name)
    }

    /// Check if a custom attribute exists.
    pub fn has_attribute(&self, name: &str) -> bool {
        attributes::has_attribute(self, name)
    }

    /// Remove a custom attribute by name.
    pub fn remove_attribute(&mut self, name: &str) -> bool {
        attributes::remove_attribute(self, name)
    }

    /// List all custom attribute names.
    pub fn attribute_names(&self) -> Vec<String> {
        attributes::attribute_names(self)
    }

    /// Get attribute metadata: (domain_name, type_name, element_count).
    pub fn attribute_info(&self, name: &str) -> Option<(String, String, usize)> {
        attributes::attribute_info(self, name)
    }

    /// Retrieve attribute data as a native Python list.
    pub fn get_attribute_data<'py>(
        &self,
        py: Python<'py>,
        name: &str,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        attributes::get_attribute_data(self, py, name)
    }

    /// Flattened vertex positions `[x0, y0, z0, x1, y1, z1, ...]`.
    pub fn get_flat_positions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_flat_positions(self, py)
    }

    /// Flattened vertex normals `[nx0, ny0, nz0, nx1, ny1, nz1, ...]`.
    pub fn get_flat_normals<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_flat_normals(self, py)
    }

    /// Flattened vertex UVs `[u0, v0, u1, v1, ...]`.
    pub fn get_flat_uvs<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_flat_uvs(self, py)
    }

    /// Triangle face indices `[i0, i1, i2, i3, i4, i5, ...]`.
    pub fn get_indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_indices(self, py)
    }

    /// Quad polygon vertex indices `[v0, v1, v2, v3, ...]`.
    pub fn get_quad_indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_quad_indices(self, py)
    }

    /// Number of quad faces recorded.
    #[getter]
    pub fn quad_count(&self) -> usize {
        attributes::quad_count(self)
    }

    /// Material slot per face.
    pub fn get_face_materials<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_face_materials(self, py)
    }

    /// Tint index per face.
    pub fn get_face_tint_indices<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        attributes::get_face_tint_indices(self, py)
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
