//! MesherConfig Python bindings.

use crate::material::PyBiomeResolver;
use crate::texture::PyBakedAtlas;
use mtk_voxel::types::{CoordinateSystem, MesherConfig};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Mesher generation configuration.
#[pyclass(name = "MesherConfig")]
#[derive(Debug, Clone, Default)]
pub struct PyMesherConfig {
    pub(crate) inner: MesherConfig,
    pub(crate) num_threads: Option<usize>,
}

#[pymethods]
impl PyMesherConfig {
    #[new]
    #[pyo3(signature = (enable_ao=true, mesh_fluids=true, z_up_coordinates=true, origin_centered=true, weld_vertices=true, num_threads=None, atlas=None, biome_resolver=None, custom_aliases=None, enable_alternate_blocks=true, enable_random_offsets=true))]
    pub fn new(
        enable_ao: bool,
        mesh_fluids: bool,
        z_up_coordinates: bool,
        origin_centered: bool,
        weld_vertices: bool,
        num_threads: Option<usize>,
        atlas: Option<&PyBakedAtlas>,
        biome_resolver: Option<&PyBiomeResolver>,
        custom_aliases: Option<HashMap<String, Vec<String>>>,
        enable_alternate_blocks: bool,
        enable_random_offsets: bool,
    ) -> Self {
        let config = MesherConfig {
            enable_ao,
            mesh_fluids,
            origin_centered,
            weld_vertices,
            coordinate_system: if z_up_coordinates {
                CoordinateSystem::ZUpRightHanded
            } else {
                CoordinateSystem::Minecraft
            },
            atlas_address_map: atlas.map(|a| Arc::new(a.inner.address_map.clone())),
            biome_resolver: biome_resolver.map(|r| Arc::new(r.inner.clone())),
            custom_aliases: custom_aliases.map(Arc::new),
            enable_alternate_blocks,
            enable_random_offsets,
            ..Default::default()
        };
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

    /// Sets or clears the BiomeResolver for face tint calculations.
    pub fn set_biome_resolver(&mut self, resolver: Option<&PyBiomeResolver>) {
        self.inner.biome_resolver = resolver.map(|r| Arc::new(r.inner.clone()));
    }

    /// Checks if a BiomeResolver is configured.
    pub fn has_biome_resolver(&self) -> bool {
        self.inner.biome_resolver.is_some()
    }

    /// Sets or clears custom texture alias mapping.
    pub fn set_custom_aliases(&mut self, aliases: Option<HashMap<String, Vec<String>>>) {
        self.inner.custom_aliases = aliases.map(Arc::new);
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
    pub fn origin_centered(&self) -> bool {
        self.inner.origin_centered
    }

    #[setter]
    pub fn set_origin_centered(&mut self, val: bool) {
        self.inner.origin_centered = val;
    }

    #[getter]
    pub fn weld_vertices(&self) -> bool {
        self.inner.weld_vertices
    }

    #[setter]
    pub fn set_weld_vertices(&mut self, val: bool) {
        self.inner.weld_vertices = val;
    }

    #[getter]
    pub fn enable_alternate_blocks(&self) -> bool {
        self.inner.enable_alternate_blocks
    }

    #[setter]
    pub fn set_enable_alternate_blocks(&mut self, val: bool) {
        self.inner.enable_alternate_blocks = val;
    }

    #[getter]
    pub fn enable_random_offsets(&self) -> bool {
        self.inner.enable_random_offsets
    }

    #[setter]
    pub fn set_enable_random_offsets(&mut self, val: bool) {
        self.inner.enable_random_offsets = val;
    }

    /// Sets explicit bounding box `(min_x, min_y, min_z, size_x, size_y, size_z)` for origin centering.
    pub fn set_bounds(
        &mut self,
        min_x: i32,
        min_y: i32,
        min_z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
    ) {
        self.inner.selection_bounds = Some(([min_x, min_y, min_z], [size_x, size_y, size_z]));
    }

    /// Clears any explicit bounding box, falling back to auto-detected bounds.
    pub fn clear_bounds(&mut self) {
        self.inner.selection_bounds = None;
    }

    #[getter]
    pub fn z_up_coordinates(&self) -> bool {
        matches!(
            self.inner.coordinate_system,
            CoordinateSystem::ZUpRightHanded
        )
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
            "<MesherConfig ao={} fluids={} centered={} weld={} coord={:?} threads={:?}>",
            self.inner.enable_ao,
            self.inner.mesh_fluids,
            self.inner.origin_centered,
            self.inner.weld_vertices,
            self.inner.coordinate_system,
            self.num_threads
        )
    }
}
