//! VoxelStorage container Python bindings.

//! # `mtk-py` Voxel Storage and Configuration Binding
//!
//! Exposes `VoxelStorage` 3D world data containers and `MesherConfig` to Python.

use pyo3::prelude::*;
use pyo3::types::PyList;

use mtk_voxel::storage::VoxelStorage;

use super::config::PyMesherConfig;

fn map_voxel_err(e: mtk_voxel::VoxelError) -> PyErr {
    match e {
        mtk_voxel::VoxelError::MissingVanillaBasePack(msg)
        | mtk_voxel::VoxelError::MissingAssets(msg) => pyo3::exceptions::PyValueError::new_err(msg),
        _ => pyo3::exceptions::PyRuntimeError::new_err(e.to_string()),
    }
}

/// Python wrapper for `VoxelStorage` sparse 3D chunk voxel container.
#[pyclass(name = "VoxelStorage")]
#[derive(Debug, Clone)]
pub struct PyVoxelStorage {
    pub(crate) inner: VoxelStorage,
}

impl Default for PyVoxelStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyVoxelStorage {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: VoxelStorage::new(),
        }
    }

    /// Pure-code generation of a canonical Minecraft debug world.
    /// Can take an optional list of blockstates, a ResourcePackStack, or an unpack directory path.
    #[staticmethod]
    #[pyo3(signature = (states=None, stack=None, path=None))]
    pub fn create_debug_world(
        states: Option<Vec<String>>,
        stack: Option<&crate::resource::PyResourcePackStack>,
        path: Option<&str>,
    ) -> PyResult<Self> {
        let storage = if let Some(st) = states {
            VoxelStorage::create_debug_world_from_states(&st)
        } else if let Some(s) = stack {
            VoxelStorage::create_debug_world_from_pack_stack(&s.inner)
        } else if let Some(p) = path {
            VoxelStorage::create_debug_world_from_dir(p)
        } else {
            VoxelStorage::create_debug_world()
        }
        .map_err(map_voxel_err)?;

        Ok(Self { inner: storage })
    }

    /// Pure-code generation from an explicit slice of blockstate strings.
    #[staticmethod]
    pub fn create_debug_world_from_states(states: Vec<String>) -> PyResult<Self> {
        let storage =
            VoxelStorage::create_debug_world_from_states(&states).map_err(map_voxel_err)?;
        Ok(Self { inner: storage })
    }

    /// Pure-code generation by enumerating blockstates across a ResourcePackStack.
    #[staticmethod]
    pub fn create_debug_world_from_pack_stack(
        stack: &crate::resource::PyResourcePackStack,
    ) -> PyResult<Self> {
        let storage = VoxelStorage::create_debug_world_from_pack_stack(&stack.inner)
            .map_err(map_voxel_err)?;
        Ok(Self { inner: storage })
    }

    /// Clears the internally cached debug storage (useful during live pack reload or tests).
    #[staticmethod]
    pub fn clear_debug_world_cache() {
        VoxelStorage::clear_debug_world_cache();
    }

    /// Sets the active 3D selection bounding box with incremental section pruning.
    pub fn set_bounds(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
    ) -> bool {
        self.inner
            .set_bounds(min_x, min_y, min_z, size_x, size_y, size_z)
    }

    /// Returns the active or calculated bounding box: `(min_x, min_y, min_z, size_x, size_y, size_z)`.
    pub fn get_bounds(&self) -> (i32, i32, i32, i32, i32, i32) {
        self.inner.get_bounds()
    }

    /// Gets the blockstate identifier string at world coordinate `(x, y, z)`.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> &str {
        self.inner.get_block(x, y, z)
    }

    /// Sets the blockstate at world coordinate `(x, y, z)` and marks dirty sections.
    #[pyo3(signature = (x, y, z, state, biome=None))]
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, state: &str, biome: Option<&str>) -> bool {
        self.inner.set_block(x, y, z, state, biome)
    }

    /// Gets current generation counter value atomically.
    pub fn get_generation(&self) -> u64 {
        self.inner.get_generation()
    }

    #[getter]
    pub fn min_x(&self) -> i32 {
        self.inner.min_x
    }

    #[getter]
    pub fn min_y(&self) -> i32 {
        self.inner.min_y
    }

    #[getter]
    pub fn min_z(&self) -> i32 {
        self.inner.min_z
    }

    #[getter]
    pub fn size_x(&self) -> i32 {
        self.inner.size_x
    }

    #[getter]
    pub fn size_y(&self) -> i32 {
        self.inner.size_y
    }

    #[getter]
    pub fn size_z(&self) -> i32 {
        self.inner.size_z
    }

    /// Returns the number of currently dirty sections requiring remeshing.
    pub fn dirty_section_count(&self) -> usize {
        self.inner.dirty_sections.len()
    }

    /// Clears all dirty section markers.
    pub fn clear_dirty_sections(&mut self) {
        self.inner.dirty_sections.clear();
    }

    /// Marks a specific section as dirty.
    pub fn mark_section_dirty(&mut self, sec_x: i32, sec_y: i32, sec_z: i32) {
        self.inner
            .dirty_sections
            .insert(glam::IVec3::new(sec_x, sec_y, sec_z));
    }

    /// Marks all existing sections dirty for a full rebuild.
    pub fn mark_all_sections_dirty(&mut self) {
        self.inner.mark_all_sections_dirty();
    }

    /// Clears all sections, biomes, and bounds.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Checks if world coordinate `(x, y, z)` falls within the active selection bounds.
    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        self.inner.contains(x, y, z)
    }

    /// Returns the biome registry identifier for block coordinate `(x, y, z)`.
    pub fn get_biome(&self, x: i32, y: i32, z: i32) -> &str {
        self.inner.get_biome(x, y, z)
    }

    /// Ingests a single 16x16x16 section snapshot packet and marks dirty sections.
    #[pyo3(signature = (sec_x, sec_y, sec_z, start_x, start_y, start_z, size_x, size_y, size_z, palette, grid_indices, biome_palette=None, biome_indices=None))]
    pub fn set_section_snapshot(
        &mut self,
        sec_x: i32,
        sec_y: i32,
        sec_z: i32,
        start_x: i32,
        start_y: i32,
        start_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: Vec<String>,
        grid_indices: Vec<u16>,
        biome_palette: Option<Vec<String>>,
        biome_indices: Option<Vec<u16>>,
    ) -> bool {
        self.inner.set_section_snapshot(
            sec_x,
            sec_y,
            sec_z,
            start_x,
            start_y,
            start_z,
            size_x,
            size_y,
            size_z,
            &palette,
            &grid_indices,
            biome_palette.as_deref(),
            biome_indices.as_deref(),
        )
    }

    /// Computes and caches CRC32 for a single section.
    pub fn calculate_and_store_section_crc(&mut self, sec_x: i32, sec_y: i32, sec_z: i32) -> u32 {
        self.inner
            .calculate_and_store_section_crc(glam::IVec3::new(sec_x, sec_y, sec_z))
    }

    /// Returns a list of all non-empty section coordinate tuples `(sx, sy, sz)`.
    pub fn get_non_empty_sections<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty(py);
        for coord in self.inner.get_all_non_empty_sections() {
            let tup = (coord.x, coord.y, coord.z);
            let _ = list.append(tup);
        }
        list
    }

    /// Returns a list of all populated non-air blocks as `((x, y, z), blockstate)`.
    pub fn get_all_blocks<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty(py);
        for (x, y, z, state) in self.inner.get_all_blocks() {
            let item = ((x, y, z), state);
            let _ = list.append(item);
        }
        list
    }

    /// Extracts an unculled point cloud containing every populated voxel point.
    #[pyo3(signature = (config=None))]
    pub fn to_point_cloud(
        &self,
        config: Option<&PyMesherConfig>,
    ) -> crate::point_cloud::PyVoxelPointCloud {
        let mesher_cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
        crate::point_cloud::PyVoxelPointCloud {
            inner: self.inner.to_point_cloud(&mesher_cfg),
        }
    }

    /// Reconstructs a `VoxelStorage` from an unculled point cloud.
    #[staticmethod]
    pub fn from_point_cloud(point_cloud: &crate::point_cloud::PyVoxelPointCloud) -> Self {
        Self {
            inner: VoxelStorage::from_point_cloud(&point_cloud.inner),
        }
    }

    /// Replaces current storage state by reconstructing from a point cloud.
    pub fn load_from_point_cloud(&mut self, point_cloud: &crate::point_cloud::PyVoxelPointCloud) {
        self.inner.load_from_point_cloud(&point_cloud.inner);
    }

    fn __repr__(&self) -> String {
        format!(
            "<VoxelStorage bounds=({}, {}, {}, size={}x{}x{}) sections={} dirty={}>",
            self.inner.min_x,
            self.inner.min_y,
            self.inner.min_z,
            self.inner.size_x,
            self.inner.size_y,
            self.inner.size_z,
            self.inner.sections.len(),
            self.inner.dirty_sections.len()
        )
    }
}
