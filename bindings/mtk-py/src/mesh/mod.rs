//! # `mtk-py` Mesh Buffer Binding
//!
//! Exposes contiguous geometry buffers (`MeshData`) to Python with zero-copy
//! buffer protocol and memoryview support for ultrafast Blender vertex injection.

pub mod attributes;
#[cfg(feature = "blender")]
pub mod memoryview;
pub mod topology;

pub use topology::analyze_mesh_hard_edges;

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyList, PyMemoryView};

use mtk_core::mesh::MeshData;
use mtk_core::polygon::FlatPolygonMesh;

pub use crate::attributes::PyAttributeDomain;
#[cfg(feature = "blender")]

/// Python-facing wrapper around contiguous `MeshData`.
#[pyclass(name = "MeshData")]
#[derive(Debug, Clone, Default)]
pub struct PyMeshData {
    pub inner: MeshData,
    pub profile: Option<mtk_core::profile::ExecutionProfile>,
}

impl From<MeshData> for PyMeshData {
    fn from(inner: MeshData) -> Self {
        Self {
            inner,
            profile: None,
        }
    }
}

impl PyMeshData {
    /// Wraps raw MeshData with no profile.
    pub fn from_mesh(inner: MeshData) -> Self {
        Self {
            inner,
            profile: None,
        }
    }

    /// Direct reference to underlying MeshData.
    pub fn inner(&self) -> &MeshData {
        &self.inner
    }

    /// Direct mutable reference to underlying MeshData.
    pub fn inner_mut(&mut self) -> &mut MeshData {
        &mut self.inner
    }

    /// Constructs a `PyMeshData` with an execution profile.
    pub fn with_profile(mut self, profile: mtk_core::profile::ExecutionProfile) -> Self {
        self.profile = Some(profile);
        self
    }
}

#[pymethods]
impl PyMeshData {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: MeshData::new(),
            profile: None,
        }
    }

    /// Creates a new `MeshData` with pre-allocated capacity.
    #[staticmethod]
    pub fn with_capacity(num_vertices: usize, num_indices: usize, num_faces: usize) -> Self {
        Self {
            inner: MeshData::with_capacity(num_vertices, num_indices, num_faces),
            profile: None,
        }
    }

    /// Execution profile of the operation that generated this mesh, if recorded.
    #[getter]
    pub fn profile(&self) -> Option<crate::profile::PyExecutionProfile> {
        self.profile.clone().map(Into::into)
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

    /// Constructs a shared-vertex polygon `MeshData` with per-loop UVs.
    #[staticmethod]
    #[pyo3(signature = (positions, loop_vertices, loop_uvs, face_loop_starts, face_loop_totals, normals=None, face_materials=None))]
    pub fn from_flat_polygons(
        positions: Vec<f32>,
        loop_vertices: Vec<u32>,
        loop_uvs: Vec<f32>,
        face_loop_starts: Vec<u32>,
        face_loop_totals: Vec<u32>,
        normals: Option<Vec<f32>>,
        face_materials: Option<Vec<u32>>,
    ) -> PyResult<Self> {
        if positions.len() % 3 != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "positions buffer length must be a multiple of 3",
            ));
        }
        if loop_uvs.len() % 2 != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "loop_uvs buffer length must be a multiple of 2",
            ));
        }
        if let Some(ref values) = normals {
            if values.len() % 3 != 0 {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "normals buffer length must be a multiple of 3",
                ));
            }
        }
        let flat = FlatPolygonMesh::from_flat_buffers(
            &positions,
            loop_vertices,
            &loop_uvs,
            face_loop_starts,
            face_loop_totals,
            face_materials.unwrap_or_default(),
        );
        let normal_vec = normals.map(|values| {
            values
                .as_chunks::<3>()
                .0
                .iter()
                .map(|v| [v[0], v[1], v[2]])
                .collect::<Vec<[f32; 3]>>()
        });
        let mesh = MeshData::from_flat_polygons(&flat, normal_vec.as_deref())
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self::from_mesh(mesh))
    }

    // -------------------------------------------------------------------------
    // Zero-Copy MemoryView Delegates (implemented in memoryview.rs)
    // -------------------------------------------------------------------------

    /// Read-only memoryview of vertex positions as raw bytes (`float32 * 3` per vertex).
    pub fn positions_memoryview<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::positions_memoryview(slf)
    }

    /// Read-only memoryview of vertex normals as raw bytes (`float32 * 3` per vertex).
    pub fn normals_memoryview<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::normals_memoryview(slf)
    }

    /// Read-only memoryview of vertex UV coordinates as raw bytes (`float32 * 2` per vertex).
    pub fn uvs_memoryview<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::uvs_memoryview(slf)
    }

    /// Flattened secondary vertex UVs `[u0, v0, u1, v1, ...]`.
    pub fn get_flat_secondary_uvs<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyList>>> {
        memoryview::get_flat_secondary_uvs(self, py)
    }

    /// Read-only memoryview of triangle indices as raw bytes (`uint32` per index).
    pub fn indices_memoryview<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::indices_memoryview(slf)
    }

    /// Read-only memoryview of quad indices as raw bytes (`uint32` per index) if quads are recorded.
    pub fn quad_indices_memoryview<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        memoryview::quad_indices_memoryview(slf)
    }

    /// Read-only memoryview of expanded per-loop UV coordinates (`float32 * 2` per loop).
    pub fn loop_uvs_memoryview<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::loop_uvs_memoryview(slf)
    }

    /// Read-only memoryview of polygon loop start offsets (`int32` per polygon).
    pub fn loop_starts_memoryview<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::loop_starts_memoryview(slf)
    }

    /// Read-only memoryview of polygon loop totals (`int32` per polygon: 4 for quads, 3 for triangles).
    pub fn loop_totals_memoryview<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::loop_totals_memoryview(slf)
    }

    /// Read-only memoryview of face material slots (`uint16` per face).
    pub fn face_materials_memoryview<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
        memoryview::face_materials_memoryview(slf)
    }

    /// Read-only memoryview of vertex RGBA colors (`float32 * 4` per vertex) if present.
    pub fn colors_memoryview<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        memoryview::colors_memoryview(slf)
    }

    /// Explicit UV layout: `"none"`, `"per_vertex"` or `"per_loop"`.
    #[getter]
    pub fn uv_layout(&self) -> &'static str {
        match self.inner.uv_layout {
            mtk_core::mesh::UvLayout::None => "none",
            mtk_core::mesh::UvLayout::PerVertex => "per_vertex",
            mtk_core::mesh::UvLayout::PerLoop => "per_loop",
        }
    }

    /// Number of quad faces recorded.
    #[getter]
    pub fn quad_count(&self) -> usize {
        self.inner.quad_count()
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
    // Custom Attributes & List Delegates (implemented in attributes.rs)
    // -------------------------------------------------------------------------

    /// Read-only zero-copy memoryview of a numeric custom attribute by name.
    pub fn attribute_memoryview<'py>(
        slf: &Bound<'py, Self>,
        name: &str,
    ) -> PyResult<Option<Bound<'py, PyMemoryView>>> {
        attributes::attribute_memoryview(slf, name)
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

    /// Build a compact `(palette, u32 indices)` view of `mtk_source_texture_key`.
    ///
    /// Hosts can inject palette-based provenance without materializing one Python
    /// string per face; the returned memoryview is a zero-copy view over an owned
    /// Rust buffer (`np.frombuffer(indices, dtype=np.uint32)` on the Python side).
    pub fn source_texture_palette_indices<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<(Vec<String>, Bound<'py, PyMemoryView>)> {
        let mut palette: Vec<String> = Vec::new();
        let mut lookup: HashMap<&str, u32> = HashMap::new();
        let mut indices: Vec<u32> = Vec::new();

        if let Some(attr) = self.inner.get_custom_attribute("mtk_source_texture_key") {
            if let mtk_core::attributes::AttributeData::String(ref values) = attr.data {
                indices.reserve(values.len());
                for value in values {
                    let next = palette.len() as u32;
                    let idx = *lookup.entry(value.as_str()).or_insert_with(|| {
                        palette.push(value.clone());
                        next
                    });
                    indices.push(idx);
                }
            }
        }

        let memoryview = crate::buffer::create_owned_u32_memoryview(py, indices)?;
        Ok((palette, memoryview))
    }

    /// Check if a custom attribute exists.
    pub fn has_attribute(&self, name: &str) -> bool {
        attributes::has_attribute(self, name)
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

    fn __repr__(&self) -> String {
        format!(
            "<MeshData vertices={} triangles={} faces={}>",
            self.vertex_count(),
            self.triangle_count(),
            self.face_count()
        )
    }
}

pub use crate::extrude::process_flat_mesh_extrude_repair;
pub use crate::subdivide::{
    adaptive_pixel_split_mesh, calculate_face_target_grid, calculate_pixel_grid_cut_factors,
    slice_face_by_pixel_grid,
};
