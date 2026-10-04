//! # `mtk-py` Voxel Point Cloud Binding
//!
//! Exposes `VoxelPointCloud` high-throughput unculled 3D voxel point cloud storage,
//! zero-copy memoryviews, bidirectional conversions with `MeshData`, and `VoxelStorage`.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList, PyMemoryView};

use mtk_voxel::storage::VoxelPointCloud;

use crate::mesh::PyMeshData;
use crate::voxel::PyVoxelStorage;

/// Python wrapper for `VoxelPointCloud` flat 3D voxel point cloud container.
#[pyclass(name = "VoxelPointCloud")]
#[derive(Debug, Clone, Default)]
pub struct PyVoxelPointCloud {
    pub(crate) inner: VoxelPointCloud,
}

#[pymethods]
impl PyVoxelPointCloud {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: VoxelPointCloud::new(),
        }
    }

    /// Creates an empty `VoxelPointCloud` with pre-allocated capacity for N points.
    #[staticmethod]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: VoxelPointCloud::with_capacity(capacity),
        }
    }

    /// Returns the total number of voxel points stored in this cloud.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Checks if the point cloud contains no voxel points.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Appends a single voxel point.
    #[pyo3(signature = (pos, block_pos, state, biome=None, light=None))]
    pub fn push(
        &mut self,
        pos: [f32; 3],
        block_pos: [i32; 3],
        state: String,
        biome: Option<String>,
        light: Option<u8>,
    ) {
        self.inner.push(
            pos,
            block_pos,
            state,
            biome.unwrap_or_else(|| "minecraft:plains".to_string()),
            light.unwrap_or(0),
        );
    }

    /// Converts this `VoxelPointCloud` into a pure-point `MeshData` with standard
    /// attributes attached on the `Point` domain (without edges or polygons).
    pub fn to_mesh_data(&self) -> PyMeshData {
        PyMeshData {
            inner: self.inner.to_mesh_data(),
        }
    }

    /// Reconstructs a `VoxelPointCloud` from a `MeshData` containing Point-domain attributes.
    #[staticmethod]
    pub fn from_mesh_data(mesh: &PyMeshData) -> PyResult<Self> {
        let cloud = VoxelPointCloud::from_mesh_data(&mesh.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: cloud })
    }

    /// Reconstructs a `VoxelStorage` from this point cloud.
    pub fn to_storage(&self) -> PyVoxelStorage {
        PyVoxelStorage {
            inner: self.inner.reconstruct_storage(),
        }
    }

    /// Returns optional bounding box `[min_x, min_y, min_z, size_x, size_y, size_z]`.
    #[getter]
    pub fn bounds(&self) -> Option<[i32; 6]> {
        self.inner.bounds
    }

    #[setter]
    pub fn set_bounds(&mut self, bounds: Option<[i32; 6]>) {
        self.inner.bounds = bounds;
    }

    /// Batch constructor from flat arrays or lists.
    #[staticmethod]
    #[pyo3(signature = (positions, block_x, block_y, block_z, block_states, biomes=None, light_levels=None, bounds=None))]
    pub fn from_arrays(
        positions: Vec<f32>,
        block_x: Vec<i32>,
        block_y: Vec<i32>,
        block_z: Vec<i32>,
        block_states: Vec<String>,
        biomes: Option<Vec<String>>,
        light_levels: Option<Vec<u8>>,
        bounds: Option<[i32; 6]>,
    ) -> PyResult<Self> {
        let count = block_x.len();
        if positions.len() != count * 3
            || block_y.len() != count
            || block_z.len() != count
            || block_states.len() != count
        {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Array lengths mismatch: positions={}, block_x={}, block_y={}, block_z={}, block_states={}",
                positions.len(),
                block_x.len(),
                block_y.len(),
                block_z.len(),
                block_states.len()
            )));
        }

        let biomes_vec = biomes.unwrap_or_else(|| vec!["minecraft:plains".to_string(); count]);
        let lights_vec = light_levels.unwrap_or_else(|| vec![0u8; count]);

        Ok(Self {
            inner: VoxelPointCloud {
                positions,
                block_x,
                block_y,
                block_z,
                block_states,
                biomes: biomes_vec,
                light_levels: lights_vec,
                bounds,
            },
        })
    }

    /// Read-only memoryview of 3D vertex positions as raw bytes (`float32 * 3` per point).
    pub fn positions_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.positions.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.positions.as_ptr() as *const u8,
                self.inner.positions.len() * std::mem::size_of::<f32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of native integer X block coordinates (`int32` per point).
    pub fn block_x_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.block_x.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.block_x.as_ptr() as *const u8,
                self.inner.block_x.len() * std::mem::size_of::<i32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of native integer Y block coordinates (`int32` per point).
    pub fn block_y_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.block_y.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.block_y.as_ptr() as *const u8,
                self.inner.block_y.len() * std::mem::size_of::<i32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of native integer Z block coordinates (`int32` per point).
    pub fn block_z_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        if self.inner.block_z.is_empty() {
            let bytes = PyBytes::new(py, &[]);
            return PyMemoryView::from(&bytes);
        }
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                self.inner.block_z.as_ptr() as *const u8,
                self.inner.block_z.len() * std::mem::size_of::<i32>(),
            )
        };
        let bytes = PyBytes::new(py, byte_slice);
        PyMemoryView::from(&bytes)
    }

    /// Read-only memoryview of composite light levels (`uint8` per point).
    pub fn light_levels_memoryview<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyMemoryView>> {
        let bytes = PyBytes::new(py, &self.inner.light_levels);
        PyMemoryView::from(&bytes)
    }

    /// Returns the list of canonical Minecraft blockstate strings.
    pub fn get_block_states<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty(py);
        for s in &self.inner.block_states {
            let _ = list.append(s);
        }
        list
    }

    /// Returns the list of biome registry identifier strings.
    pub fn get_biomes<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty(py);
        for b in &self.inner.biomes {
            let _ = list.append(b);
        }
        list
    }

    /// Returns a list of all block coordinates as `(x, y, z)` tuples.
    pub fn get_block_positions<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty(py);
        for i in 0..self.inner.len() {
            let item = (self.inner.block_x[i], self.inner.block_y[i], self.inner.block_z[i]);
            let _ = list.append(item);
        }
        list
    }

    fn __repr__(&self) -> String {
        format!("<VoxelPointCloud len={}>", self.inner.len())
    }
}
