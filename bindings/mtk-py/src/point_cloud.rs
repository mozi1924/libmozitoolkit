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

enum BorrowedF32Buffer {
    Py(pyo3::buffer::PyBuffer<f32>),
    Owned(Vec<f32>),
}

impl BorrowedF32Buffer {
    fn extract(py_obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(buf) = pyo3::buffer::PyBuffer::<f32>::get(py_obj) {
            if buf.is_c_contiguous() {
                return Ok(Self::Py(buf));
            }
        }
        let vec = py_obj.extract::<Vec<f32>>()?;
        Ok(Self::Owned(vec))
    }

    fn as_slice(&self) -> &[f32] {
        match self {
            Self::Py(buf) => unsafe {
                std::slice::from_raw_parts(buf.buf_ptr() as *const f32, buf.item_count())
            },
            Self::Owned(vec) => vec.as_slice(),
        }
    }
}

enum BorrowedI32Buffer {
    Py(pyo3::buffer::PyBuffer<i32>),
    Owned(Vec<i32>),
}

impl BorrowedI32Buffer {
    fn extract(py_obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(buf) = pyo3::buffer::PyBuffer::<i32>::get(py_obj) {
            if buf.is_c_contiguous() {
                return Ok(Self::Py(buf));
            }
        }
        let vec = py_obj.extract::<Vec<i32>>()?;
        Ok(Self::Owned(vec))
    }

    fn as_slice(&self) -> &[i32] {
        match self {
            Self::Py(buf) => unsafe {
                std::slice::from_raw_parts(buf.buf_ptr() as *const i32, buf.item_count())
            },
            Self::Owned(vec) => vec.as_slice(),
        }
    }
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

    /// High-performance batch constructor from palette indices using buffer protocol / memoryview.
    #[staticmethod]
    #[pyo3(signature = (positions, block_x, block_y, block_z, state_indices, state_palette, biome_indices=None, biome_palette=None, light_levels=None, bounds=None))]
    pub fn from_indexed_arrays(
        positions: &Bound<'_, PyAny>,
        block_x: &Bound<'_, PyAny>,
        block_y: &Bound<'_, PyAny>,
        block_z: &Bound<'_, PyAny>,
        state_indices: &Bound<'_, PyAny>,
        state_palette: Vec<String>,
        biome_indices: Option<&Bound<'_, PyAny>>,
        biome_palette: Option<Vec<String>>,
        light_levels: Option<Vec<u8>>,
        bounds: Option<[i32; 6]>,
    ) -> PyResult<Self> {
        let pos_buf = BorrowedF32Buffer::extract(positions)?;
        let bx_buf = BorrowedI32Buffer::extract(block_x)?;
        let by_buf = BorrowedI32Buffer::extract(block_y)?;
        let bz_buf = BorrowedI32Buffer::extract(block_z)?;
        let st_buf = BorrowedI32Buffer::extract(state_indices)?;

        let bm_buf = match biome_indices {
            Some(obj) => Some(BorrowedI32Buffer::extract(obj)?),
            None => None,
        };

        let pos_slice = pos_buf.as_slice();
        let bx_slice = bx_buf.as_slice();
        let by_slice = by_buf.as_slice();
        let bz_slice = bz_buf.as_slice();
        let st_slice = st_buf.as_slice();
        let bm_slice = bm_buf.as_ref().map(|b| b.as_slice());

        let cloud = VoxelPointCloud::from_indexed_arrays(
            pos_slice.to_vec(),
            bx_slice.to_vec(),
            by_slice.to_vec(),
            bz_slice.to_vec(),
            st_slice,
            &state_palette,
            bm_slice,
            biome_palette.as_deref(),
            light_levels,
            bounds,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(Self { inner: cloud })
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
    pub fn light_levels_memoryview<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyMemoryView>> {
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
            let item = (
                self.inner.block_x[i],
                self.inner.block_y[i],
                self.inner.block_z[i],
            );
            let _ = list.append(item);
        }
        list
    }

    /// Builds a unique state palette and corresponding per-point palette indices.
    pub fn build_state_palette_indices(&self) -> (Vec<String>, Vec<i32>) {
        self.inner.build_state_palette_indices()
    }

    /// Builds a unique biome palette and corresponding per-point palette indices.
    pub fn build_biome_palette_indices(&self) -> (Vec<String>, Vec<i32>) {
        self.inner.build_biome_palette_indices()
    }

    /// Copies 3D positions directly into a destination memory pointer (`float32 * 3` per point).
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_positions_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        crate::blender::direct_copy_point_cloud_positions_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Copies discrete integer block X coordinates directly into a destination pointer (`int32` per point).
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_block_x_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        crate::blender::direct_copy_point_cloud_block_x_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Copies discrete integer block Y coordinates directly into a destination pointer (`int32` per point).
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_block_y_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        crate::blender::direct_copy_point_cloud_block_y_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Copies discrete integer block Z coordinates directly into a destination pointer (`int32` per point).
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_block_z_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        crate::blender::direct_copy_point_cloud_block_z_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Converts block states into a palette and copies int32 palette indices into a destination pointer.
    /// Returns `(palette, bytes_written)`.
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_state_indices_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<(Vec<String>, usize)> {
        crate::blender::direct_copy_point_cloud_state_indices_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Converts biomes into a palette and copies int32 palette indices into a destination pointer.
    /// Returns `(biome_palette, bytes_written)`.
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_biome_indices_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<(Vec<String>, usize)> {
        crate::blender::direct_copy_point_cloud_biome_indices_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Copies light levels directly into a destination pointer (`uint8` per point).
    #[cfg(feature = "blender")]
    #[pyo3(signature = (dst_ptr, max_bytes=None))]
    pub fn direct_copy_light_levels_to_ptr(
        &self,
        dst_ptr: usize,
        max_bytes: Option<usize>,
    ) -> PyResult<usize> {
        crate::blender::direct_copy_point_cloud_light_levels_to_ptr(self, dst_ptr, max_bytes)
    }

    /// Batch copies all available voxel point cloud attributes directly into destination pointers in a single call.
    /// Returns `(Option<state_palette>, Option<biome_palette>)`.
    #[cfg(feature = "blender")]
    #[pyo3(signature = (pos_ptr=None, block_x_ptr=None, block_y_ptr=None, block_z_ptr=None, state_idx_ptr=None, biome_idx_ptr=None, light_ptr=None))]
    pub fn direct_copy_to_ptrs(
        &self,
        pos_ptr: Option<usize>,
        block_x_ptr: Option<usize>,
        block_y_ptr: Option<usize>,
        block_z_ptr: Option<usize>,
        state_idx_ptr: Option<usize>,
        biome_idx_ptr: Option<usize>,
        light_ptr: Option<usize>,
    ) -> PyResult<(Option<Vec<String>>, Option<Vec<String>>)> {
        crate::blender::direct_copy_point_cloud_to_ptrs(
            self,
            pos_ptr,
            block_x_ptr,
            block_y_ptr,
            block_z_ptr,
            state_idx_ptr,
            biome_idx_ptr,
            light_ptr,
        )
    }

    fn __repr__(&self) -> String {
        format!("<VoxelPointCloud len={}>", self.inner.len())
    }
}
