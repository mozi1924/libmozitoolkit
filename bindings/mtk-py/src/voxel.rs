//! # `mtk-py` Voxel Storage and Configuration Binding
//!
//! Exposes `VoxelStorage` 3D world data containers and `MesherConfig` to Python.

use pyo3::prelude::*;
use pyo3::types::PyList;

use mtk_voxel::storage::VoxelStorage;
use mtk_voxel::types::{CoordinateSystem, MesherConfig};

/// Python wrapper for `VoxelStorage` sparse 3D chunk voxel container.
#[pyclass(name = "VoxelStorage")]
#[derive(Debug, Clone)]
pub struct PyVoxelStorage {
    pub(crate) inner: VoxelStorage,
}

#[pymethods]
impl PyVoxelStorage {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: VoxelStorage::new(),
        }
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
        self.inner.set_bounds(min_x, min_y, min_z, size_x, size_y, size_z)
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

    /// Ingests a full binary/snapshot packet with block palette and optional biome stream.
    #[pyo3(signature = (min_x, min_y, min_z, size_x, size_y, size_z, palette, grid_indices, biome_palette=None, biome_indices=None))]
    pub fn set_full_snapshot(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: Vec<String>,
        grid_indices: Vec<u16>,
        biome_palette: Option<Vec<String>>,
        biome_indices: Option<Vec<u16>>,
    ) {
        self.inner.set_full_snapshot(
            min_x,
            min_y,
            min_z,
            size_x,
            size_y,
            size_z,
            &palette,
            &grid_indices,
            biome_palette.as_deref(),
            biome_indices.as_deref(),
        );
    }

    /// Applies a batch of block delta modifications.
    ///
    /// `changes`: List of tuples `(x, y, z, new_blockstate)`
    /// Returns: List of tuples `(x, y, z, old_blockstate, new_canonical_blockstate)`
    pub fn apply_delta_update(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        changes: Vec<(i32, i32, i32, String)>,
    ) -> Vec<(i32, i32, i32, String, String)> {
        let borrowed_changes: Vec<(i32, i32, i32, &str)> = changes
            .iter()
            .map(|(x, y, z, s)| (*x, *y, *z, s.as_str()))
            .collect();
        self.inner
            .apply_delta_update(min_x, min_y, min_z, &borrowed_changes)
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

    /// Returns the list of dirty section coordinate tuples `(sx, sy, sz)`.
    pub fn get_dirty_sections<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let list = PyList::empty(py);
        for coord in &self.inner.dirty_sections {
            let _ = list.append((coord.x, coord.y, coord.z));
        }
        list
    }

    /// Clears all dirty section markers.
    pub fn clear_dirty_sections(&mut self) {
        self.inner.dirty_sections.clear();
    }

    /// Marks a specific section as dirty.
    pub fn mark_section_dirty(&mut self, sec_x: i32, sec_y: i32, sec_z: i32) {
        self.inner.dirty_sections.insert(glam::IVec3::new(sec_x, sec_y, sec_z));
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

    /// Checks if incoming full snapshot data matches current memory state without mutations.
    pub fn is_snapshot_identical(
        &self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        palette: Vec<String>,
        grid_indices: Vec<u16>,
    ) -> bool {
        self.inner.is_snapshot_identical(
            min_x,
            min_y,
            min_z,
            size_x,
            size_y,
            size_z,
            &palette,
            &grid_indices,
        )
    }

    /// Compares server section CRC32 hashes with local ones and returns mismatched section coordinates.
    pub fn validate_manifest(
        &mut self,
        server_sections: Vec<(i32, i32, i32, u32)>,
        existing_section_meshes: Option<Vec<(i32, i32, i32)>>,
    ) -> Vec<(i32, i32, i32)> {
        let set: Option<std::collections::HashSet<glam::IVec3>> = existing_section_meshes.map(|list| {
            list.into_iter()
                .map(|(x, y, z)| glam::IVec3::new(x, y, z))
                .collect()
        });

        self.inner
            .validate_manifest(&server_sections, set.as_ref())
            .into_iter()
            .map(|c| (c.x, c.y, c.z))
            .collect()
    }

    /// Checks if a CRC32 matches the canonical empty air CRC for this section's clamped volume.
    pub fn is_empty_section_crc(&self, sec_x: i32, sec_y: i32, sec_z: i32, crc_val: u32) -> bool {
        self.inner.is_empty_section_crc(glam::IVec3::new(sec_x, sec_y, sec_z), crc_val)
    }

    /// Computes and caches CRC32 for a single section.
    pub fn calculate_and_store_section_crc(&mut self, sec_x: i32, sec_y: i32, sec_z: i32) -> u32 {
        self.inner.calculate_and_store_section_crc(glam::IVec3::new(sec_x, sec_y, sec_z))
    }

    /// Recomputes CRC32 for all sections currently loaded.
    pub fn recalculate_all_section_crcs(&mut self) {
        self.inner.recalculate_all_section_crcs();
    }

    /// Exports manifest metadata (bounds and CRC map) as JSON string for scene persistence.
    pub fn export_manifest_metadata(&self) -> String {
        self.inner.export_manifest_metadata().to_string()
    }

    /// Imports manifest metadata from JSON string.
    pub fn import_manifest_metadata(&mut self, json_str: &str) -> bool {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            self.inner.import_manifest_metadata(&val)
        } else {
            false
        }
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

/// Mesher generation configuration.
#[pyclass(name = "MesherConfig")]
#[derive(Debug, Clone)]
pub struct PyMesherConfig {
    pub(crate) inner: MesherConfig,
    pub(crate) num_threads: Option<usize>,
}

use std::sync::Arc;

use crate::texture::PyBakedAtlas;

#[pymethods]
impl PyMesherConfig {
    #[new]
    #[pyo3(signature = (enable_ao=true, mesh_fluids=true, z_up_coordinates=true, num_threads=None, atlas=None))]
    pub fn new(
        enable_ao: bool,
        mesh_fluids: bool,
        z_up_coordinates: bool,
        num_threads: Option<usize>,
        atlas: Option<&PyBakedAtlas>,
    ) -> Self {
        let mut config = MesherConfig::default();
        config.enable_ao = enable_ao;
        config.mesh_fluids = mesh_fluids;
        config.coordinate_system = if z_up_coordinates {
            CoordinateSystem::ZUpRightHanded
        } else {
            CoordinateSystem::Minecraft
        };
        config.atlas_address_map = atlas.map(|a| Arc::new(a.inner.address_map.clone()));
        Self {
            inner: config,
            num_threads,
        }
    }

    /// Sets or clears the Atlas address mapping table for UV remapping and material slotting.
    pub fn set_atlas(&mut self, atlas: Option<&PyBakedAtlas>) {
        self.inner.atlas_address_map = atlas.map(|a| Arc::new(a.inner.address_map.clone()));
    }

    /// Checks if an Atlas address map is configured.
    pub fn has_atlas(&self) -> bool {
        self.inner.atlas_address_map.is_some()
    }

    #[getter]
    pub fn enable_ao(&self) -> bool {
        self.inner.enable_ao
    }

    #[setter]
    pub fn set_enable_ao(&mut self, val: bool) {
        self.inner.enable_ao = val;
    }

    #[getter]
    pub fn mesh_fluids(&self) -> bool {
        self.inner.mesh_fluids
    }

    #[setter]
    pub fn set_mesh_fluids(&mut self, val: bool) {
        self.inner.mesh_fluids = val;
    }

    #[getter]
    pub fn z_up_coordinates(&self) -> bool {
        matches!(self.inner.coordinate_system, CoordinateSystem::ZUpRightHanded)
    }

    #[setter]
    pub fn set_z_up_coordinates(&mut self, val: bool) {
        self.inner.coordinate_system = if val {
            CoordinateSystem::ZUpRightHanded
        } else {
            CoordinateSystem::Minecraft
        };
    }

    #[getter]
    pub fn num_threads(&self) -> Option<usize> {
        self.num_threads
    }

    #[setter]
    pub fn set_num_threads(&mut self, val: Option<usize>) {
        self.num_threads = val;
    }

    fn __repr__(&self) -> String {
        format!(
            "<MesherConfig ao={} fluids={} coord={:?} threads={:?}>",
            self.inner.enable_ao,
            self.inner.mesh_fluids,
            self.inner.coordinate_system,
            self.num_threads
        )
    }
}
